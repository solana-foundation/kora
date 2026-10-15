import { PublicKey, VersionedTransaction } from "@solana/web3.js";
import { checkAbuseBudgets } from "./abuse.mjs";
import { checkAppAccess, checkAppReady, settingsFingerprint } from "./access.mjs";
import { Refusal } from "./errors.mjs";
import { appendInstructions, decodeTransaction, describeMessage, messageHash, resolveLookupTables } from "./message.mjs";
import { lamportsToMicroUsdc, splitPayment } from "./money.mjs";
import { buildPaymentInstructions, usdcAccountOf } from "./payments.mjs";
import {
  baseFeeLamports,
  candidateTokenAccounts,
  checkAllowedPrograms,
  checkCaps,
  checkFeePayerExposure,
  checkFeePayerSlot,
  checkNoDurableNonce,
  checkSimulatedOutflow,
  checkUser,
  readComputeBudget,
  tokenAccountOwner,
} from "./policy.mjs";

/** A blockhash is accepted for about 150 slots; this assumes the caller fetched it just now. */
export const BLOCKHASH_LIFETIME_MS = 60_000;

const NO_PAYMENT = { networkCost: 0n, markup: 0n, noirwire: 0n, app: 0n, total: 0n };
const TOKEN_ACCOUNT_AMOUNT_OFFSET = 64;

const text = (value) => value.toString();

/** Every token account among the transaction's accounts that the fee payer owns. */
async function tokenAccountsOwnedByFeePayer(chain, desc, feePayer, feePayerBytes) {
  const keys = candidateTokenAccounts(desc, feePayer);
  const infos = await chain.getMultipleAccountsInfo(keys.map((key) => new PublicKey(key)));
  const owned = new Set();
  infos.forEach((info, i) => {
    const owner = tokenAccountOwner(info);
    if (owner && owner.equals(feePayerBytes)) owned.add(keys[i]);
  });
  return owned;
}

/** What the fee payer would lose running this transaction, signature and priority fees aside. */
async function simulatedLoss(chain, tx, feePayer) {
  const before = await chain.getBalance(new PublicKey(feePayer));
  const simulation = await chain.simulate(tx, feePayer);
  if (simulation.err) {
    throw new Refusal("simulation_failed", `the transaction fails when simulated: ${JSON.stringify(simulation.err)}`);
  }
  if (simulation.feePayerLamports === null) throw new Refusal("simulation_failed", "the simulation did not report the fee payer");
  return before > simulation.feePayerLamports ? before - simulation.feePayerLamports : 0n;
}

async function requireUsdcFunds(chain, config, user, total) {
  const account = await chain.getAccountInfo(usdcAccountOf(config.usdcMint, user));
  if (!account) throw new Refusal("no_usdc_account", "the user has no USDC account");
  const amount = Buffer.from(account.data).readBigUInt64LE(TOKEN_ACCOUNT_AMOUNT_OFFSET);
  if (amount < total) throw new Refusal("insufficient_usdc", `the user needs ${total} micro-USDC and has ${amount}`);
}

function parseUser(user) {
  try {
    if (typeof user !== "string") throw new Error("not text");
    return new PublicKey(user);
  } catch {
    throw new Refusal("bad_request", "user must be a public key");
  }
}

/**
 * Validates an unsigned transaction for an app, measures what it costs the fee payer, and
 * returns it with the USDC payment appended, plus a quote pinned to the hash of the final
 * message. The quote also records the worst case the send will reserve.
 */
export async function prepare(ctx, { transaction, user, appId, origin, clientIp }) {
  const { config, chain, store, quotes } = ctx;
  const now = ctx.now();
  const feePayer = config.feePayer.toBase58();
  const userKey = parseUser(user);
  if (typeof appId !== "string" || appId.length === 0) throw new Refusal("bad_request", "app is required");
  const app = await store.getApp(appId);
  if (!app) throw new Refusal("unknown_app", "no app is registered with that id");
  checkAppAccess(app, origin);
  checkAppReady(app, config);
  await checkAbuseBudgets(ctx, { user: userKey.toBase58(), ip: clientIp, app });

  const tx = decodeTransaction(transaction, config.caps.maxTxBytes);
  const message = tx.message;
  if (message.staticAccountKeys[0].toBase58() !== feePayer) {
    throw new Refusal("wrong_fee_payer", `the transaction's fee payer must be ${feePayer}`);
  }
  const tables = await resolveLookupTables(chain, message);
  const desc = describeMessage(message, tables);

  checkFeePayerSlot(desc, feePayer);
  checkUser(desc, userKey.toBase58(), feePayer);
  checkNoDurableNonce(desc);
  checkAllowedPrograms(desc, app.allowedPrograms);
  const budget = readComputeBudget(desc);
  checkCaps(desc, budget, config.caps);
  const owned = await tokenAccountsOwnedByFeePayer(chain, desc, feePayer, config.feePayer.toBytes());
  checkFeePayerExposure(desc, feePayer, config.feePayer.toBytes(), owned);

  const loss = await simulatedLoss(chain, tx, feePayer);
  checkSimulatedOutflow(loss, config.caps);
  const { signatureFee, priorityFee } = baseFeeLamports(desc, budget);
  const fees = signatureFee + priorityFee;
  const networkCostLamports = fees + loss;

  let split = NO_PAYMENT;
  let finalMessage = message;
  if (config.charge) {
    const networkCost = lamportsToMicroUsdc(networkCostLamports, await ctx.readPrice());
    split = splitPayment({ networkCost, markupBps: app.markupBps });
    await requireUsdcFunds(chain, config, userKey, split.total);
    const payments = buildPaymentInstructions({
      user: userKey,
      mint: config.usdcMint,
      noirwireAccount: config.paymentUsdcAccount,
      appAccount: split.app > 0n ? usdcAccountOf(config.usdcMint, new PublicKey(app.payoutWallet)) : null,
      split,
    });
    finalMessage = appendInstructions(message, tables, payments);
  }

  const finalTransaction = new VersionedTransaction(finalMessage);
  const serialized = finalTransaction.serialize();
  if (serialized.length > config.caps.maxTxBytes) {
    throw new Refusal("too_large", `with the payment added the transaction is ${serialized.length} bytes, the limit is ${config.caps.maxTxBytes}`);
  }

  const hash = messageHash(finalMessage);
  const expiresAt = now + Math.min(config.quoteTtlSeconds * 1000, BLOCKHASH_LIFETIME_MS);
  quotes.put(hash, {
    user: userKey.toBase58(),
    appId: app.id,
    settingsFingerprint: settingsFingerprint(app),
    blockhash: finalMessage.recentBlockhash,
    networkCostLamports,
    split,
    // Failing on chain costs the fees. Landing costs the fees and the simulated outflow.
    lossLamports: fees,
    sponsoredLamports: networkCostLamports,
    expiresAt,
  });

  return {
    transaction: Buffer.from(serialized).toString("base64"),
    quote: {
      networkCostMicroUsdc: text(split.networkCost),
      markupMicroUsdc: text(split.markup),
      appMicroUsdc: text(split.app),
      noirwireMicroUsdc: text(split.noirwire),
      totalMicroUsdc: text(split.total),
    },
    expiresAt: new Date(expiresAt).toISOString(),
  };
}
