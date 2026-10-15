import { VersionedTransaction } from "@solana/web3.js";
import { checkAbuseBudgets, reservationBudgets } from "./abuse.mjs";
import { checkAppAccess, settingsFingerprint } from "./access.mjs";
import { broadcastSigned, RELEASED, recordedOutcome, unknown } from "./broadcast.mjs";
import { requireReconciliationHealthy } from "./confirm.mjs";
import { Refusal } from "./errors.mjs";
import { messageBytes, messageHash, userSignaturesValid, verifyEd25519 } from "./message.mjs";
import { base58Encode } from "./units.mjs";

const REFUSAL_MESSAGES = {
  loss_ceiling: "the relayer has reached its daily loss limit, try again after 00:00 UTC",
  wallet_loss_budget: "this wallet's transactions cost the relayer too much today, try again after 00:00 UTC",
  ip_loss_budget: "transactions from this address cost the relayer too much today, try again after 00:00 UTC",
  app_loss_budget: "this app's transactions cost the relayer too much today, try again after 00:00 UTC",
  app_daily_cap: "this app has reached its daily sponsorship cap, try again after 00:00 UTC",
};

/** The fee payer's signature on a relayer-signed transaction, if it really is our message signed by our fee payer. */
function verifiedFeePayerSignature(signedBase64, hash, feePayer) {
  let signed;
  try {
    signed = VersionedTransaction.deserialize(Buffer.from(signedBase64, "base64"));
  } catch {
    return null;
  }
  const signature = signed.signatures[0];
  if (messageHash(signed.message) !== hash || !signature) return null;
  return verifyEd25519(signature, messageBytes(signed.message), feePayer.toBytes()) ? signature : null;
}

/**
 * Sends a user-signed transaction. Everything is decided from the quote found by the hash of
 * the submitted message, so every limit applies to the user and app that quote was made for,
 * whoever holds signer slot 1. Resolves to `{ status, body }`; throws a Refusal when nothing
 * was signed.
 */
export async function send(ctx, { tx, transaction, clientIp = null, origin }) {
  const { config, chain, store, quotes, kora } = ctx;
  const hash = messageHash(tx.message);

  const recorded = await store.getSend(hash);
  if (recorded?.status === "signed") return broadcastSigned(ctx, recorded);
  if (recorded) return recordedOutcome(recorded);

  const quote = quotes.peek(hash);
  if (!quote) {
    throw new Refusal("no_quote", "no live quote for this exact transaction; it was changed, already used, or has expired. Call prepare again");
  }
  if (!ctx.limiters.user.allow(quote.user)) throw new Refusal("rate_limited", "too many requests, slow down");
  const app = await store.getApp(quote.appId);
  if (!app) throw new Refusal("unknown_app", "no app is registered with that id");
  checkAppAccess(app, origin);
  if (settingsFingerprint(app) !== quote.settingsFingerprint) {
    throw new Refusal("app_settings_changed", "the app's settings changed after this transaction was prepared. Call prepare again");
  }
  await checkAbuseBudgets(ctx, { user: quote.user, ip: clientIp, app });
  if (!userSignaturesValid(tx)) throw new Refusal("bad_signature", "a required signature is missing or does not match the transaction");
  await requireReconciliationHealthy(ctx);
  if (!(await chain.isBlockhashValid(quote.blockhash))) {
    throw new Refusal("quote_expired", "the transaction's blockhash has expired. Call prepare again");
  }

  if (!quotes.take(hash)) throw new Refusal("no_quote", "no live quote for this exact transaction");
  const reservation = await store.reserve(
    {
      messageHash: hash,
      appId: app.id,
      userWallet: quote.user,
      clientIp,
      blockhash: quote.blockhash,
      networkCostLamports: quote.networkCostLamports,
      networkCost: quote.split.networkCost,
      markup: quote.split.markup,
      noirwire: quote.split.noirwire,
      app: quote.split.app,
      lossLamports: quote.lossLamports,
      sponsoredLamports: quote.sponsoredLamports,
    },
    reservationBudgets(ctx, app),
  );
  if (reservation.status === "exists") return recordedOutcome(await store.getSend(hash));
  if (reservation.status === "refused") {
    quotes.put(hash, quote);
    throw new Refusal(reservation.code, REFUSAL_MESSAGES[reservation.code]);
  }

  let signedBase64;
  try {
    signedBase64 = await kora.signTransaction(transaction);
  } catch (error) {
    if (error instanceof Refusal) {
      await store.recordStatus(hash, { status: "rejected", ...RELEASED });
      throw error;
    }
    await store.recordStatus(hash, { status: "unknown" });
    return unknown(hash);
  }

  const feePayerSignature = verifiedFeePayerSignature(signedBase64, hash, config.feePayer);
  if (!feePayerSignature) {
    await store.recordStatus(hash, { status: "rejected", ...RELEASED });
    throw new Refusal("kora_rejected", "the relayer returned a transaction that is not the one that was quoted");
  }
  const signature = base58Encode(feePayerSignature);
  // The signed bytes are stored before any broadcast: from here on this exact transaction may land.
  await store.recordStatus(hash, { status: "signed", signature, signedTransaction: signedBase64 });
  return broadcastSigned(ctx, { messageHash: hash, signature, signedTransaction: signedBase64 });
}
