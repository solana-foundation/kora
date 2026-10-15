import assert from "node:assert/strict";
import { test } from "node:test";
import { ed25519 } from "@noble/curves/ed25519";
import { ComputeBudgetProgram, Keypair, PublicKey, SendTransactionError, SystemProgram, VersionedTransaction } from "@solana/web3.js";
import { createAssociatedTokenAccountIdempotentInstruction } from "@solana/spl-token";
import { sweepOnce } from "../src/confirm.mjs";
import { Refusal, UnknownOutcome } from "../src/errors.mjs";
import { messageHash } from "../src/message.mjs";
import { usdcAccountOf } from "../src/payments.mjs";
import { buildServer } from "../src/server.mjs";
import { base58Encode } from "../src/units.mjs";
import {
  accountInfo,
  base64Of,
  buildTransaction,
  fundedUser,
  lookupTable,
  makeContext,
  programCall,
  seedApp,
  seedRow,
  TOKEN_PROGRAM,
  tokenAccountData,
  wallet,
} from "./helpers.mjs";

const sign = (keypair, text) => base58Encode(ed25519.sign(new TextEncoder().encode(text), keypair.secretKey.slice(0, 32)));

async function setup({ charge = true, config = {}, ctx: ctxOverrides, app: appOverrides = {} } = {}) {
  const made = makeContext({ config: { charge, ...config }, ctx: ctxOverrides });
  const server = await buildServer(made.ctx);
  const user = fundedUser(made.chain, made.config);
  const app = await seedApp(made.ctx, appOverrides);
  const destination = wallet().publicKey;
  const post = (url, payload, headers) => server.inject({ method: "POST", url, payload, headers });
  const tx = (instructions = [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: destination, lamports: 1000 })], options = {}) =>
    base64Of(buildTransaction({ payer: made.config.feePayer, instructions, ...options }));
  const prepare = (transaction = tx(), extra = {}, headers) => post("/v1/prepare", { transaction, user: user.publicKey.toBase58(), app: app.id, ...extra }, headers);
  const userSigned = (prepared, signers = [user]) => {
    const signed = VersionedTransaction.deserialize(Buffer.from(prepared.transaction, "base64"));
    signed.sign(signers);
    return base64Of(signed);
  };
  const prepareAndSign = async (extra = {}, headers) => {
    const response = await prepare(tx(), extra, headers);
    assert.equal(response.statusCode, 200, response.body);
    const prepared = response.json();
    return { prepared, signed: userSigned(prepared), hash: messageHash(VersionedTransaction.deserialize(Buffer.from(prepared.transaction, "base64")).message) };
  };
  const loginAs = (keypair) => ({ authorization: `Bearer ${made.ctx.sessions.create(keypair.publicKey.toBase58()).token}` });
  const owner = Keypair.generate();
  return { ...made, server, user, app, destination, post, tx, prepare, userSigned, prepareAndSign, loginAs, owner };
}

const instructionData = (base64, index) => Buffer.from(VersionedTransaction.deserialize(Buffer.from(base64, "base64")).message.compiledInstructions[index].data);

test("health and config have the contract's shape and no secrets", async () => {
  const t = await setup();
  t.chain.balance = 100_000_000n;
  const { refreshFeePayerHealth } = await import("../src/health.mjs");
  await refreshFeePayerHealth(t.ctx);
  const health = (await t.server.inject({ method: "GET", url: "/v1/health" })).json();
  assert.deepEqual(Object.keys(health).sort(), ["feePayer", "network", "ok"]);
  assert.equal(health.ok, true);
  assert.equal(health.network, "devnet");
  assert.deepEqual(health.feePayer, { state: "ready", balanceLamports: "100000000", checkedAt: new Date(t.clock.ms).toISOString() });

  const config = (await t.server.inject({ method: "GET", url: "/v1/config" })).json();
  assert.deepEqual(config, {
    network: "devnet",
    feePayer: t.config.feePayer.toBase58(),
    usdcMint: t.config.usdcMint.toBase58(),
    charge: true,
    markup: { defaultBps: 10_000, maxBps: 30_000 },
    split: { appBps: 8000, noirwireBps: 2000 },
  });
  const text = JSON.stringify([health, config]);
  for (const secret of ["api-key", "hmac-secret", "postgres://", "rpc.invalid", "kora.invalid"]) assert.ok(!text.includes(secret));
});

test("any origin may call from a browser, with the Authorization header", async () => {
  const t = await setup();
  const preflight = await t.server.inject({
    method: "OPTIONS",
    url: "/v1/apps/app_x",
    headers: { origin: "https://console.example", "access-control-request-method": "PUT", "access-control-request-headers": "authorization,content-type" },
  });
  assert.equal(preflight.headers["access-control-allow-origin"], "https://console.example");
  assert.match(preflight.headers["access-control-allow-methods"], /PUT/);
  assert.match(preflight.headers["access-control-allow-headers"], /authorization/i);
});

test("errors are { error, code } and unknown routes are 404", async () => {
  const t = await setup();
  const missing = await t.server.inject({ method: "GET", url: "/v1/nothing" });
  assert.equal(missing.statusCode, 404);
  assert.deepEqual(Object.keys(missing.json()).sort(), ["code", "error"]);
  assert.equal((await t.post("/v1/prepare", { user: "x" })).json().code, "bad_request");
  const notJson = await t.server.inject({ method: "POST", url: "/v1/prepare", payload: "{oops", headers: { "content-type": "application/json" } });
  assert.equal(notJson.statusCode, 400);
});

test("an app is required: missing, unknown, paused and not ready are refused with their codes", async () => {
  const t = await setup();
  assert.equal((await t.prepare(t.tx(), { app: undefined })).json().code, "bad_request");
  const unknown = await t.prepare(t.tx(), { app: "app_doesnotexi" });
  assert.equal(unknown.statusCode, 404);
  assert.equal(unknown.json().code, "unknown_app");

  const paused = await setup({ app: { paused: true } });
  const pausedResponse = await paused.prepare();
  assert.equal(pausedResponse.statusCode, 403);
  assert.equal(pausedResponse.json().code, "app_paused");

  const noPayout = await setup({ app: { payoutWallet: null } });
  const notReady = await noPayout.prepare();
  assert.equal(notReady.statusCode, 403);
  assert.equal(notReady.json().code, "app_not_ready");

  const free = await setup({ charge: false, app: { payoutWallet: null } });
  assert.equal((await free.prepare()).statusCode, 200, "with charging off an app needs no payout wallet");
});

test("with charging off nothing is appended, nothing is owed and no price is read", async () => {
  let priceReads = 0;
  const t = await setup({ charge: false, ctx: { readPrice: async () => (priceReads += 1) } });
  const stranger = Keypair.generate();
  const transaction = base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: stranger.publicKey, toPubkey: t.destination, lamports: 5 })] }));
  const response = await t.post("/v1/prepare", { transaction, user: stranger.publicKey.toBase58(), app: t.app.id });
  assert.equal(response.statusCode, 200, response.body);
  const body = response.json();
  assert.deepEqual(Object.keys(body).sort(), ["expiresAt", "quote", "transaction"]);
  assert.deepEqual(body.quote, { networkCostMicroUsdc: "0", markupMicroUsdc: "0", appMicroUsdc: "0", noirwireMicroUsdc: "0", totalMicroUsdc: "0" });
  assert.equal(VersionedTransaction.deserialize(Buffer.from(body.transaction, "base64")).message.compiledInstructions.length, 1);
  assert.equal(priceReads, 0);
});

test("prepare appends the app's payment: NoirWire gets cost plus its fifth, the app gets the rest", async () => {
  const t = await setup();
  const body = (await t.prepare()).json();
  // two signatures = 10,000 lamports at 150 USDC per SOL = 1,500 micro-USDC; default markup 100% = 1,500
  assert.deepEqual(body.quote, { networkCostMicroUsdc: "1500", markupMicroUsdc: "1500", appMicroUsdc: "1200", noirwireMicroUsdc: "300", totalMicroUsdc: "3000" });
  const decoded = VersionedTransaction.deserialize(Buffer.from(body.transaction, "base64"));
  assert.equal(decoded.message.compiledInstructions.length, 3);
  assert.equal(instructionData(body.transaction, 1)[0], 12, "TransferChecked");
  assert.equal(instructionData(body.transaction, 1).readBigUInt64LE(1), 1800n);
  assert.equal(instructionData(body.transaction, 2).readBigUInt64LE(1), 1200n);
  const keys = decoded.message.staticAccountKeys.map((k) => k.toBase58());
  assert.ok(keys.includes(t.config.paymentUsdcAccount.toBase58()));
  assert.ok(keys.includes(usdcAccountOf(t.config.usdcMint, new PublicKey(t.app.payoutWallet)).toBase58()));
});

test("a custom markup is honoured and a zero markup adds no app transfer", async () => {
  const half = await setup({ app: { markupBps: 5000 } });
  const body = (await half.prepare()).json();
  assert.deepEqual(body.quote, { networkCostMicroUsdc: "1500", markupMicroUsdc: "750", appMicroUsdc: "600", noirwireMicroUsdc: "150", totalMicroUsdc: "2250" });
  assert.equal(instructionData(body.transaction, 1).readBigUInt64LE(1), 1650n);

  const zero = await setup({ app: { markupBps: 0 } });
  const free = (await zero.prepare()).json();
  assert.deepEqual(free.quote, { networkCostMicroUsdc: "1500", markupMicroUsdc: "0", appMicroUsdc: "0", noirwireMicroUsdc: "0", totalMicroUsdc: "1500" });
  assert.equal(VersionedTransaction.deserialize(Buffer.from(free.transaction, "base64")).message.compiledInstructions.length, 2);
});

test("the simulated loss is charged: an account opening costs its rent", async () => {
  const t = await setup();
  t.chain.loss = 2_039_280n;
  assert.equal((await t.prepare()).json().quote.networkCostMicroUsdc, "307392");
});

test("a priced transaction needs an explicit compute limit; with one, the priority fee is charged on the final message", async () => {
  const t = await setup();
  const price = ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 20_000 });
  const transfer = SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 });
  const noLimit = await t.prepare(t.tx([price, transfer]));
  assert.equal(noLimit.statusCode, 400);
  assert.equal(noLimit.json().code, "compute_limit_required");

  const withLimit = await t.prepare(t.tx([ComputeBudgetProgram.setComputeUnitLimit({ units: 100_000 }), price, transfer]));
  // 10,000 signature lamports + 2,000 priority lamports = 12,000 lamports = 1,800 micro-USDC
  assert.equal(withLimit.json().quote.networkCostMicroUsdc, "1800");
});

test("the default configuration refuses any priority fee", async () => {
  const t = await setup({ config: { caps: { maxSignatures: 3, maxTxBytes: 1232, maxComputeUnitPrice: 0, maxPriorityFeeLamports: 0, maxComputeUnitLimit: 1_400_000, maxFeePayerOutflowLamports: 2_200_000 } } });
  const priced = t.tx([
    ComputeBudgetProgram.setComputeUnitLimit({ units: 100_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1 }),
    SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 }),
  ]);
  assert.equal((await t.prepare(priced)).json().code, "priority_fee_too_high");
  assert.equal((await t.prepare()).statusCode, 200);
});

test("a v0 transaction with a lookup table is prepared and keeps using the table; a deactivating table is refused", async () => {
  const t = await setup();
  const target = wallet().publicKey;
  const program = wallet().publicKey;
  const table = lookupTable([target, program]);
  t.chain.tables.set(table.key.toBase58(), table);
  const transaction = t.tx([programCall(program, [{ pubkey: t.user.publicKey, isSigner: true }, { pubkey: target }])], { tables: [table] });
  const response = await t.prepare(transaction);
  assert.equal(response.statusCode, 200, response.body);
  const decoded = VersionedTransaction.deserialize(Buffer.from(response.json().transaction, "base64"));
  assert.equal(decoded.message.addressTableLookups.length, 1);

  const dying = lookupTable([target, program], { active: false });
  t.chain.tables.set(dying.key.toBase58(), dying);
  const refused = await t.prepare(t.tx([programCall(program, [{ pubkey: t.user.publicKey, isSigner: true }, { pubkey: target }])], { tables: [dying] }));
  assert.equal(refused.json().code, "lookup_table_inactive");
});

test("a program instruction that receives the fee payer, or a token account it owns, is refused", async () => {
  const t = await setup();
  const program = wallet().publicKey;
  const direct = await t.prepare(t.tx([programCall(program, [{ pubkey: t.user.publicKey, isSigner: true }, { pubkey: t.config.feePayer }])]));
  assert.equal(direct.statusCode, 400);
  assert.equal(direct.json().code, "fee_payer_exposed");
  assert.equal(t.chain.sent.length, 0);

  const owned = wallet().publicKey;
  t.chain.setAccount(owned, accountInfo(TOKEN_PROGRAM, tokenAccountData({ mint: t.config.usdcMint, owner: t.config.feePayer })));
  const viaToken = await t.prepare(t.tx([programCall(program, [{ pubkey: t.user.publicKey, isSigner: true }, { pubkey: owned }])]));
  assert.equal(viaToken.json().code, "fee_payer_exposed");
});

test("the fee payer may fund an account creation", async () => {
  const t = await setup();
  t.chain.loss = 2_039_280n;
  const ix = [createAssociatedTokenAccountIdempotentInstruction(t.config.feePayer, wallet().publicKey, t.user.publicKey, wallet().publicKey), SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 })];
  assert.equal((await t.prepare(t.tx(ix))).statusCode, 200);
});

test("refusals: wrong fee payer, user not a signer, durable nonce, simulation failure, outflow", async () => {
  const t = await setup();
  const otherPayer = base64Of(buildTransaction({ payer: wallet().publicKey, instructions: [SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 })] }));
  assert.equal((await t.prepare(otherPayer)).json().code, "wrong_fee_payer");
  assert.equal((await t.post("/v1/prepare", { transaction: t.tx(), user: wallet().publicKey.toBase58(), app: t.app.id })).json().code, "user_not_signer");
  const nonce = t.tx([SystemProgram.nonceAdvance({ noncePubkey: wallet().publicKey, authorizedPubkey: t.user.publicKey }), SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 })]);
  assert.equal((await t.prepare(nonce)).json().code, "durable_nonce");
  t.chain.simulationError = { InstructionError: [0, "Custom"] };
  const failing = await t.prepare();
  assert.equal(failing.statusCode, 422);
  assert.equal(failing.json().code, "simulation_failed");
  t.chain.simulationError = null;
  t.chain.loss = 3_000_000n;
  assert.equal((await t.prepare()).json().code, "outflow_too_high");
});

test("a stale price refuses the request with 503", async () => {
  const t = await setup({ ctx: { readPrice: async () => { throw new Refusal("no_price", "no usable reference SOL price: it is 600 seconds old"); } } });
  const response = await t.prepare();
  assert.equal(response.statusCode, 503);
  assert.equal(response.json().code, "no_price");
});

test("the user needs a USDC account with enough in it when charging", async () => {
  const t = await setup();
  const poor = fundedUser(t.chain, t.config, 100n);
  const tx = base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: poor.publicKey, toPubkey: t.destination, lamports: 1 })] }));
  assert.equal((await t.post("/v1/prepare", { transaction: tx, user: poor.publicKey.toBase58(), app: t.app.id })).json().code, "insufficient_usdc");
  const nobody = Keypair.generate();
  const tx2 = base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: nobody.publicKey, toPubkey: t.destination, lamports: 1 })] }));
  assert.equal((await t.post("/v1/prepare", { transaction: tx2, user: nobody.publicKey.toBase58(), app: t.app.id })).json().code, "no_usdc_account");
});

test("the final transaction must still fit in a packet", async () => {
  const t = await setup({ config: { caps: { maxSignatures: 3, maxTxBytes: 300, maxComputeUnitPrice: 0, maxPriorityFeeLamports: 0, maxComputeUnitLimit: 1_400_000, maxFeePayerOutflowLamports: 2_200_000 } } });
  assert.equal((await t.prepare()).json().code, "too_large");
});

test("an app's allowed programs are enforced on prepare", async () => {
  const allowed = wallet().publicKey;
  const t = await setup({ app: { allowedPrograms: [allowed.toBase58()] } });
  const ok = await t.prepare(t.tx([programCall(allowed, [{ pubkey: t.user.publicKey, isSigner: true }])]));
  assert.equal(ok.statusCode, 200, ok.body);
  const refused = await t.prepare(t.tx([programCall(wallet().publicKey, [{ pubkey: t.user.publicKey, isSigner: true }])]));
  assert.equal(refused.statusCode, 403);
  assert.equal(refused.json().code, "program_not_allowed");
});

test("allowed origins: a listed or absent Origin passes, another is refused, on prepare and on send", async () => {
  const t = await setup({ app: { allowedOrigins: ["https://app.example"] } });
  assert.equal((await t.prepare(t.tx(), {}, { origin: "https://app.example" })).statusCode, 200);
  assert.equal((await t.prepare()).statusCode, 200, "a server sends no Origin");
  const refused = await t.prepare(t.tx(), {}, { origin: "https://evil.example" });
  assert.equal(refused.statusCode, 403);
  assert.equal(refused.json().code, "origin_not_allowed");

  const { signed } = await t.prepareAndSign();
  const badSend = await t.post("/v1/send", { transaction: signed }, { origin: "https://evil.example" });
  assert.equal(badSend.json().code, "origin_not_allowed");
  assert.equal(t.ctx.kora.calls.length, 0);
  assert.equal((await t.post("/v1/send", { transaction: signed }, { origin: "https://app.example" })).statusCode, 200);
});

test("prepare then send: Kora signs, the signed bytes are stored, the broadcast is accepted, and a repeat returns the same outcome", async () => {
  const t = await setup();
  const { prepared, signed, hash } = await t.prepareAndSign();
  const response = await t.post("/v1/send", { transaction: signed });
  assert.equal(response.statusCode, 200, response.body);
  const body = response.json();
  assert.deepEqual(Object.keys(body).sort(), ["signature", "status"]);
  assert.equal(body.status, "sent");
  assert.match(body.signature, /^[1-9A-HJ-NP-Za-km-z]{80,90}$/);

  assert.equal(t.ctx.kora.calls.length, 1);
  assert.equal(t.chain.sent.length, 1);
  const broadcast = VersionedTransaction.deserialize(t.chain.sent[0]);
  assert.equal(messageHash(broadcast.message), hash);
  assert.ok(broadcast.signatures[0].some((byte) => byte !== 0));
  const row = await t.ctx.store.getSend(hash);
  assert.equal(row.status, "sent");
  assert.equal(row.signedTransaction, t.chain.sent[0].toString("base64"));
  assert.equal(row.clientIp, "127.0.0.1");
  assert.equal(prepared.quote.totalMicroUsdc, "3000");

  const again = await t.post("/v1/send", { transaction: signed });
  assert.deepEqual(again.json(), body);
  assert.equal(t.ctx.kora.calls.length, 1);
  assert.equal(t.chain.sent.length, 1);
});

test("a message changed after prepare has no quote and reaches neither Kora nor the chain", async () => {
  const t = await setup();
  const { prepared } = await t.prepareAndSign();
  const original = VersionedTransaction.deserialize(Buffer.from(prepared.transaction, "base64"));
  for (const lamports of [1000, 9_999_999]) {
    const tampered = buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports })] });
    tampered.message.recentBlockhash = original.message.recentBlockhash;
    tampered.sign([t.user]);
    const response = await t.post("/v1/send", { transaction: base64Of(tampered) });
    assert.equal(response.statusCode, 404);
    assert.equal(response.json().code, "no_quote");
  }
  const payment = original.message.compiledInstructions[1];
  payment.data = Buffer.from(payment.data);
  payment.data.writeBigUInt64LE(1n, 1);
  original.sign([t.user]);
  assert.equal((await t.post("/v1/send", { transaction: base64Of(original) })).json().code, "no_quote");
  assert.equal(t.ctx.kora.calls.length, 0);
  assert.equal(t.chain.sent.length, 0);
});

test("send needs the user's signature and leaves the quote usable", async () => {
  const t = await setup();
  const { prepared, signed } = await t.prepareAndSign();
  const unsigned = await t.post("/v1/send", { transaction: prepared.transaction });
  assert.equal(unsigned.statusCode, 403);
  assert.equal(unsigned.json().code, "bad_signature");
  assert.equal(t.ctx.kora.calls.length, 0);
  assert.equal((await t.post("/v1/send", { transaction: signed })).statusCode, 200);
});

test("a quote expires, and dies with its blockhash", async () => {
  const t = await setup();
  const { signed } = await t.prepareAndSign();
  t.clock.ms += 61_000;
  assert.equal((await t.post("/v1/send", { transaction: signed })).json().code, "no_quote");

  const u = await setup();
  const second = await u.prepareAndSign();
  u.chain.blockhashValid = false;
  assert.equal((await u.post("/v1/send", { transaction: second.signed })).json().code, "quote_expired");
  assert.equal(u.ctx.kora.calls.length, 0);
});

test("an ambiguous answer from Kora is an unknown outcome and is never retried", async () => {
  const t = await setup();
  const { prepared, signed, hash } = await t.prepareAndSign();
  t.ctx.kora.behaviour = new UnknownOutcome("socket hang up");
  const response = await t.post("/v1/send", { transaction: signed });
  assert.equal(response.statusCode, 202);
  assert.deepEqual(response.json(), { outcome: "unknown", messageHash: hash });
  t.ctx.kora.behaviour = null;
  const again = await t.post("/v1/send", { transaction: signed });
  assert.equal(again.statusCode, 202);
  assert.equal(t.ctx.kora.calls.length, 1);
  assert.equal(t.chain.sent.length, 0);
  assert.ok(prepared);
});

test("a refusal from Kora releases the reservation and is answered the same way next time", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.ctx.kora.behaviour = new Refusal("kora_rejected", "the relayer refused to sign: policy");
  const response = await t.post("/v1/send", { transaction: signed });
  assert.equal(response.statusCode, 422);
  assert.equal(response.json().code, "kora_rejected");
  const row = await t.ctx.store.getSend(hash);
  assert.equal(row.status, "rejected");
  assert.equal(row.lossLamports, 0n);
  assert.equal(row.sponsoredLamports, 0n);
  t.ctx.kora.behaviour = null;
  assert.equal((await t.post("/v1/send", { transaction: signed })).json().code, "rejected");
  assert.equal(t.ctx.kora.calls.length, 1);
});

test("a Kora answer that is not our message signed by our fee payer is not broadcast", async () => {
  const t = await setup();
  const { signed } = await t.prepareAndSign();
  t.ctx.kora.signTransaction = async () => base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 77 })] }));
  assert.equal((await t.post("/v1/send", { transaction: signed })).statusCode, 422);
  assert.equal(t.chain.sent.length, 0);
});

test("signed is a distinct state: an unclear broadcast stays signed, and a repeat send rebroadcasts the identical bytes without signing again", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.chain.sendError = new UnknownOutcome("fetch failed");
  const response = await t.post("/v1/send", { transaction: signed });
  assert.equal(response.statusCode, 202);
  const body = response.json();
  assert.deepEqual(Object.keys(body).sort(), ["messageHash", "outcome", "signature"]);
  const row = await t.ctx.store.getSend(hash);
  assert.equal(row.status, "signed", "not marked sent before a node accepted it");
  assert.ok(row.signedTransaction);
  assert.equal(t.chain.sent.length, 0);

  t.chain.sendError = null;
  const repeat = await t.post("/v1/send", { transaction: signed });
  assert.equal(repeat.statusCode, 200);
  assert.deepEqual(repeat.json(), { signature: body.signature, status: "sent" });
  assert.equal(t.chain.sent[0].toString("base64"), row.signedTransaction, "the identical bytes");
  assert.equal(t.ctx.kora.calls.length, 1, "never signed again");
  assert.equal((await t.ctx.store.getSend(hash)).status, "sent");
});

test("a node's refusal of one broadcast never releases a signed row: it stays reserved until the blockhash is gone and the signature is absent", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.chain.sendError = new SendTransactionError({ action: "send", signature: "", transactionMessage: "Blockhash not found" });
  const response = await t.post("/v1/send", { transaction: signed });
  assert.equal(response.statusCode, 202);
  assert.equal(response.json().outcome, "unknown");
  assert.ok(response.json().signature);
  let row = await t.ctx.store.getSend(hash);
  assert.equal(row.status, "signed");
  assert.equal(row.lossLamports, 10_000n, "still reserved");
  assert.equal(row.broadcastLeaseUntil, null, "the lease is given back so the sweep can try again");

  await sweepOnce(t.ctx); // blockhash still valid: tries again, the node still refuses
  assert.equal((await t.ctx.store.getSend(hash)).status, "signed");

  t.chain.blockhashValid = false;
  t.chain.statuses.set(response.json().signature, { err: null });
  await sweepOnce(t.ctx);
  assert.equal((await t.ctx.store.getSend(hash)).status, "landed", "an earlier attempt had landed after all");

  const u = await setup();
  const second = await u.prepareAndSign();
  u.chain.sendError = new SendTransactionError({ action: "send", signature: "", transactionMessage: "Blockhash not found" });
  await u.post("/v1/send", { transaction: second.signed });
  u.chain.blockhashValid = false;
  await sweepOnce(u.ctx);
  row = await u.ctx.store.getSend(second.hash);
  assert.equal(row.status, "expired");
  assert.equal(row.lossLamports, 0n);
  assert.equal(row.sponsoredLamports, 0n);
});

test("broadcast attempts are serialized per message: concurrent sends of a signed row reach the network once", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.chain.sendError = new UnknownOutcome("fetch failed");
  assert.equal((await t.post("/v1/send", { transaction: signed })).statusCode, 202);
  t.chain.sendError = null;

  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  let reached = 0;
  t.chain.sendRawTransaction = async (raw) => {
    reached += 1;
    await gate;
    t.chain.sent.push(raw);
    return "sig";
  };
  const attempts = [t.post("/v1/send", { transaction: signed }), t.post("/v1/send", { transaction: signed }), t.post("/v1/send", { transaction: signed })];
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(reached, 1, "the other two found the lease taken and never touched the network");
  release();
  const results = await Promise.all(attempts);
  assert.deepEqual(results.map((r) => r.statusCode).sort(), [200, 202, 202]);
  assert.equal(t.chain.sent.length, 1);
  assert.equal((await t.ctx.store.getSend(hash)).status, "sent");
  assert.equal(t.ctx.kora.calls.length, 1);
});

test("a lease that expired can be taken again", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.chain.sendError = new UnknownOutcome("fetch failed");
  await t.post("/v1/send", { transaction: signed });
  assert.equal(await t.ctx.store.claimBroadcast(hash, t.clock.ms, 30_000), true);
  assert.equal(await t.ctx.store.claimBroadcast(hash, t.clock.ms + 29_999, 30_000), false);
  assert.equal(await t.ctx.store.claimBroadcast(hash, t.clock.ms + 30_000, 30_000), true);
});

test("a Kora answer that never came is reconciled: held while the blockhash lives, released once it is gone", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.ctx.kora.behaviour = new UnknownOutcome("socket hang up");
  assert.equal((await t.post("/v1/send", { transaction: signed })).statusCode, 202);
  assert.equal((await t.ctx.store.getSend(hash)).lossLamports, 10_000n);

  await sweepOnce(t.ctx);
  assert.equal((await t.ctx.store.getSend(hash)).status, "unknown", "the blockhash is still valid");
  assert.equal((await t.ctx.store.getSend(hash)).lossLamports, 10_000n);

  t.chain.blockhashValid = false;
  await sweepOnce(t.ctx);
  const row = await t.ctx.store.getSend(hash);
  assert.equal(row.status, "expired");
  assert.equal(row.lossLamports, 0n);
  assert.equal(row.sponsoredLamports, 0n);
  assert.match(row.failureReason, /cannot land/);
});

test("settings that changed after prepare make send refuse with 409 until the caller prepares again", async () => {
  for (const change of [
    { markupBps: 5000 },
    { payoutWallet: wallet().publicKey.toBase58() },
    { allowedPrograms: [wallet().publicKey.toBase58()] },
  ]) {
    const t = await setup();
    const { signed } = await t.prepareAndSign();
    await t.ctx.store.updateApp(t.app.id, change);
    const refused = await t.post("/v1/send", { transaction: signed });
    assert.equal(refused.statusCode, 409, JSON.stringify(change));
    assert.equal(refused.json().code, "app_settings_changed");
    assert.equal(t.ctx.kora.calls.length, 0);
    assert.equal(t.chain.sent.length, 0);

    const again = await t.prepareAndSign();
    assert.equal((await t.post("/v1/send", { transaction: again.signed })).statusCode, 200);
  }
});

test("settings that do not affect a quote (name, origins, cap, order of programs) do not invalidate it", async () => {
  const [a, b] = [wallet().publicKey.toBase58(), wallet().publicKey.toBase58()];
  const t = await setup({ app: { allowedPrograms: [a, b] } });
  const { signed } = await t.prepareAndSign();
  await t.ctx.store.updateApp(t.app.id, { name: "Renamed", dailyCapLamports: 20_000_000n, allowedPrograms: [b, a] });
  assert.equal((await t.post("/v1/send", { transaction: signed })).statusCode, 200);
});

test("every send limit applies to the quote's user, not to signer slot 1", async () => {
  const t = await setup({ config: { limits: { globalPerMinute: 100_000, ipPerMinute: 1000, userPerMinute: 2, registerPerMinute: 1000 } } });
  const helper = Keypair.generate();
  const program = wallet().publicKey;
  const threeSigners = () =>
    t.tx([programCall(program, [{ pubkey: helper.publicKey, isSigner: true }, { pubkey: t.user.publicKey, isSigner: true }]), SystemProgram.transfer({ fromPubkey: t.user.publicKey, toPubkey: t.destination, lamports: 1 })]);
  const first = await t.prepare(threeSigners());
  assert.equal(first.statusCode, 200, first.body);
  const decoded = VersionedTransaction.deserialize(Buffer.from(first.json().transaction, "base64"));
  assert.equal(decoded.message.header.numRequiredSignatures, 3);
  const [, slot1, slot2] = decoded.message.staticAccountKeys.map((key) => key.toBase58());
  assert.equal(slot1, helper.publicKey.toBase58());
  assert.equal(slot2, t.user.publicKey.toBase58());

  const signedTx = VersionedTransaction.deserialize(Buffer.from(first.json().transaction, "base64"));
  signedTx.sign([helper, t.user]);
  const calls = t.chain.rpcCalls;
  t.ctx.limiters.user.allow(t.user.publicKey.toBase58()); // the user is now at its limit of 2
  const limited = await t.post("/v1/send", { transaction: base64Of(signedTx) });
  assert.equal(limited.statusCode, 429);
  assert.equal(t.chain.rpcCalls, calls, "refused before any RPC call");
  assert.equal(t.ctx.limiters.user.windows.has(slot1), false, "the slot 1 signer was never counted");

  t.ctx.limiters.user.windows.clear();
  const ok = await t.post("/v1/send", { transaction: base64Of(signedTx) });
  assert.equal(ok.statusCode, 200, ok.body);
  assert.equal((await t.ctx.store.getSend(messageHash(signedTx.message))).userWallet, t.user.publicKey.toBase58());
});

test("a wallet over its loss budget cannot send even from a slot it does not hold", async () => {
  const t = await setup();
  const { signed } = await t.prepareAndSign();
  await seedRow(t.ctx.store, { appId: t.app.id, user: t.user.publicKey.toBase58(), ip: "5.5.5.5", status: "failed", loss: 50_000n });
  const refused = await t.post("/v1/send", { transaction: signed });
  assert.equal(refused.statusCode, 429);
  assert.equal(refused.json().code, "wallet_loss_budget");
  assert.equal(t.ctx.kora.calls.length, 0);
});

test("concurrent sends cannot all pass the global ceiling: the worst case is reserved at admission", async () => {
  const t = await setup({ config: { dailyLossCeilingLamports: 25_000n } });
  const prepared = [];
  for (let i = 0; i < 4; i += 1) {
    const user = fundedUser(t.chain, t.config);
    const transaction = base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: t.destination, lamports: 1 + i })] }));
    const response = await t.post("/v1/prepare", { transaction, user: user.publicKey.toBase58(), app: t.app.id });
    assert.equal(response.statusCode, 200, response.body);
    prepared.push(t.userSigned(response.json(), [user]));
  }
  const results = await Promise.all(prepared.map((signed) => t.post("/v1/send", { transaction: signed })));
  const codes = results.map((r) => r.statusCode).sort();
  assert.deepEqual(codes, [200, 200, 503, 503], "10,000 lamports of fees each: two fit under 25,000");
  assert.ok(results.filter((r) => r.statusCode === 503).every((r) => r.json().code === "loss_ceiling"));
  assert.equal(t.ctx.kora.calls.length, 2);
});

test("the app's daily cap counts landed sponsorship too", async () => {
  const t = await setup({ app: { dailyCapLamports: 15_000n } });
  const first = await t.prepareAndSign();
  const sent = await t.post("/v1/send", { transaction: first.signed });
  t.chain.statuses.set(sent.json().signature, { err: null });
  await sweepOnce(t.ctx);
  assert.equal((await t.ctx.store.getSend(first.hash)).status, "landed");
  const user2 = fundedUser(t.chain, t.config);
  const tx2 = base64Of(buildTransaction({ payer: t.config.feePayer, instructions: [SystemProgram.transfer({ fromPubkey: user2.publicKey, toPubkey: t.destination, lamports: 5 })] }));
  const prepared = await t.post("/v1/prepare", { transaction: tx2, user: user2.publicKey.toBase58(), app: t.app.id });
  assert.equal(prepared.statusCode, 200, "10,000 so far is under the 15,000 cap");
  const refused = await t.post("/v1/send", { transaction: t.userSigned(prepared.json(), [user2]) });
  assert.equal(refused.statusCode, 429);
  assert.equal(refused.json().code, "app_daily_cap");
});

test("sends are refused with 503 while reconciliation is unhealthy", async () => {
  const t = await setup();
  for (let i = 0; i < 21; i += 1) await seedRow(t.ctx.store, { appId: t.app.id, user: `u${i}`, ip: `i${i}`, status: "sent" });
  t.clock.ms += 11 * 60_000 - 30_000;
  const { signed } = await t.prepareAndSign();
  t.clock.ms += 31_000;
  const refused = await t.post("/v1/send", { transaction: signed });
  assert.equal(refused.statusCode, 503);
  assert.equal(refused.json().code, "reconciliation_unhealthy");
  assert.equal(t.ctx.kora.calls.length, 0);
});

test("rate limits per IP and per wallet apply before any RPC call", async () => {
  const t = await setup({ config: { limits: { globalPerMinute: 100_000, ipPerMinute: 2, userPerMinute: 1000, registerPerMinute: 1000 } } });
  await t.prepare();
  await t.prepare();
  const calls = t.chain.rpcCalls;
  const limited = await t.prepare();
  assert.equal(limited.statusCode, 429);
  assert.equal(limited.json().code, "rate_limited");
  assert.equal(t.chain.rpcCalls, calls);

  const byUser = await setup({ config: { limits: { globalPerMinute: 100_000, ipPerMinute: 1000, userPerMinute: 1, registerPerMinute: 1000 } } });
  assert.equal((await byUser.prepare()).statusCode, 200);
  const calls2 = byUser.chain.rpcCalls;
  assert.equal((await byUser.prepare()).statusCode, 429);
  assert.equal(byUser.chain.rpcCalls, calls2);
});

test("a global ingress limit sits in front of every per-key limit", async () => {
  const t = await setup({ config: { limits: { globalPerMinute: 3, ipPerMinute: 1000, userPerMinute: 1000, registerPerMinute: 1000 } } });
  for (let i = 0; i < 3; i += 1) assert.equal((await t.server.inject({ method: "GET", url: "/v1/config" })).statusCode, 200);
  const refused = await t.server.inject({ method: "GET", url: "/v1/config" });
  assert.equal(refused.statusCode, 503);
  assert.equal(refused.json().code, "overloaded");
});

test("behind one trusted proxy the client is the right-most forwarded address; a forged leading entry changes nothing", async () => {
  const t = await setup({ config: { trustedProxyHops: 1, limits: { globalPerMinute: 100_000, ipPerMinute: 1000, userPerMinute: 1000, registerPerMinute: 1 } } });
  const challenge = (forwarded) =>
    t.server.inject({ method: "POST", url: "/v1/auth/challenge", remoteAddress: "10.0.0.1", headers: { "x-forwarded-for": forwarded }, payload: { wallet: t.owner.publicKey.toBase58() } });
  assert.equal((await challenge("203.0.113.9")).statusCode, 200);
  const forged = await challenge("1.2.3.4, 203.0.113.9");
  assert.equal(forged.statusCode, 429, "the forged first entry did not make a new client");
  assert.equal((await challenge("1.2.3.4, 203.0.113.10")).statusCode, 200, "a different real client has its own allowance");
});

test("with no trusted proxy the forwarding header is ignored entirely", async () => {
  const t = await setup({ config: { trustedProxyHops: 0, limits: { globalPerMinute: 100_000, ipPerMinute: 1000, userPerMinute: 1000, registerPerMinute: 1 } } });
  const challenge = (forwarded) =>
    t.server.inject({ method: "POST", url: "/v1/auth/challenge", headers: { "x-forwarded-for": forwarded }, payload: { wallet: t.owner.publicKey.toBase58() } });
  assert.equal((await challenge("1.1.1.1")).statusCode, 200);
  assert.equal((await challenge("2.2.2.2")).statusCode, 429);
});

async function signIn(t, keypair = t.owner) {
  const wallet = keypair.publicKey.toBase58();
  const challenge = (await t.post("/v1/auth/challenge", { wallet })).json();
  const response = await t.post("/v1/auth/session", { wallet, nonce: challenge.nonce, signature: sign(keypair, challenge.message) });
  return { challenge, response };
}

test("sign-in: challenge text, signature, a 12 hour bearer token, and logout", async () => {
  const t = await setup();
  const { challenge, response } = await signIn(t);
  assert.deepEqual(Object.keys(challenge).sort(), ["expiresAt", "message", "nonce"]);
  assert.match(challenge.message, new RegExp(t.owner.publicKey.toBase58()));
  assert.match(challenge.message, /Network: devnet/);
  assert.match(challenge.message, /not a transaction/);
  assert.equal(response.statusCode, 200, response.body);
  const { token, expiresAt } = response.json();
  assert.equal(Date.parse(expiresAt) - t.clock.ms, 12 * 3_600_000);
  const headers = { authorization: `Bearer ${token}` };
  assert.equal((await t.server.inject({ method: "GET", url: "/v1/me/apps", headers })).statusCode, 200);
  const out = await t.server.inject({ method: "DELETE", url: "/v1/auth/session", headers });
  assert.equal(out.statusCode, 204);
  assert.equal((await t.server.inject({ method: "GET", url: "/v1/me/apps", headers })).statusCode, 401);
});

test("sign-in: a wrong signature, another wallet's signature, a reused or unknown nonce, and an expired one are refused", async () => {
  const t = await setup();
  const wallet = t.owner.publicKey.toBase58();
  const challenge = (await t.post("/v1/auth/challenge", { wallet })).json();
  const forged = await t.post("/v1/auth/session", { wallet, nonce: challenge.nonce, signature: sign(Keypair.generate(), challenge.message) });
  assert.equal(forged.statusCode, 403);
  const other = Keypair.generate();
  const stolen = await t.post("/v1/auth/session", { wallet: other.publicKey.toBase58(), nonce: challenge.nonce, signature: sign(other, challenge.message) });
  assert.equal(stolen.statusCode, 403, "a challenge belongs to the wallet it was issued for");
  const good = await t.post("/v1/auth/session", { wallet, nonce: challenge.nonce, signature: sign(t.owner, challenge.message) });
  assert.equal(good.statusCode, 200, "failed attempts did not burn the nonce");
  const replay = await t.post("/v1/auth/session", { wallet, nonce: challenge.nonce, signature: sign(t.owner, challenge.message) });
  assert.equal(replay.statusCode, 403);
  assert.equal((await t.post("/v1/auth/session", { wallet, nonce: "made-up", signature: "x" })).statusCode, 403);

  const late = (await t.post("/v1/auth/challenge", { wallet })).json();
  t.clock.ms += 6 * 60_000;
  assert.equal((await t.post("/v1/auth/session", { wallet, nonce: late.nonce, signature: sign(t.owner, late.message) })).statusCode, 403);
  assert.equal((await t.post("/v1/auth/challenge", { wallet: "nonsense" })).statusCode, 400);
});

test("owner routes need a bearer token", async () => {
  const t = await setup();
  for (const [method, url] of [["GET", "/v1/me/apps"], ["POST", "/v1/apps"], ["GET", `/v1/apps/${t.app.id}`], ["PATCH", `/v1/apps/${t.app.id}`], ["PUT", `/v1/apps/${t.app.id}/payout`], ["GET", `/v1/apps/${t.app.id}/stats`], ["GET", `/v1/apps/${t.app.id}/activity`]]) {
    const response = await t.server.inject({ method, url, payload: method === "GET" ? undefined : {} });
    assert.equal(response.statusCode, 401, `${method} ${url}`);
    assert.equal(response.json().code, "unauthenticated");
  }
  assert.equal((await t.server.inject({ method: "GET", url: "/v1/me/apps", headers: { authorization: "Bearer nonsense" } })).statusCode, 401);
});

test("apps: created with defaults, listed, fetched by the owner only, and updated", async () => {
  const t = await setup({ charge: true });
  const headers = t.loginAs(t.owner);
  const created = await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: "My Wallet" } });
  assert.equal(created.statusCode, 201, created.body);
  const app = created.json();
  assert.match(app.id, /^app_[a-z0-9]{10}$/);
  assert.deepEqual(
    { ...app, id: undefined, createdAt: undefined },
    {
      id: undefined,
      name: "My Wallet",
      owner: t.owner.publicKey.toBase58(),
      network: "devnet",
      payoutWallet: null,
      markupBps: 10_000,
      allowedOrigins: [],
      allowedPrograms: [],
      dailyCapLamports: "10000000",
      paused: false,
      ready: false,
      createdAt: undefined,
      firstTransactionAt: null,
    },
  );
  assert.equal(app.createdAt, new Date(t.clock.ms).toISOString());

  assert.deepEqual((await t.server.inject({ method: "GET", url: "/v1/me/apps", headers })).json().apps.map((a) => a.id), [app.id]);
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}`, headers })).statusCode, 200);
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}`, headers: t.loginAs(Keypair.generate()) })).statusCode, 404, "others get 404");
  assert.deepEqual((await t.server.inject({ method: "GET", url: "/v1/me/apps", headers: t.loginAs(Keypair.generate()) })).json().apps, []);

  const patched = await t.server.inject({
    method: "PATCH",
    url: `/v1/apps/${app.id}`,
    headers,
    payload: { name: "Renamed", markupBps: 20_000, allowedOrigins: ["https://app.example"], allowedPrograms: [wallet().publicKey.toBase58()], dailyCapLamports: "5000000", paused: true },
  });
  assert.equal(patched.statusCode, 200, patched.body);
  const view = patched.json();
  assert.equal(view.name, "Renamed");
  assert.equal(view.markupBps, 20_000);
  assert.deepEqual(view.allowedOrigins, ["https://app.example"]);
  assert.equal(view.dailyCapLamports, "5000000");
  assert.equal(view.paused, true);
  assert.equal(view.ready, false);
});

test("apps: invalid settings are refused and another owner's app cannot be changed", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  const app = (await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: "A" } })).json();
  const patch = (payload, h = headers) => t.server.inject({ method: "PATCH", url: `/v1/apps/${app.id}`, headers: h, payload });
  for (const payload of [{ markupBps: 30_001 }, { allowedOrigins: ["nope"] }, { allowedPrograms: ["nope"] }, { dailyCapLamports: "999999999999" }, { paused: "yes" }, { name: "" }]) {
    assert.equal((await patch(payload)).statusCode, 400, JSON.stringify(payload));
  }
  assert.equal((await patch({ markupBps: 1 }, t.loginAs(Keypair.generate()))).statusCode, 404);
  assert.equal((await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: {} })).statusCode, 400);
});

test("an owner can have at most 20 apps", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  for (let i = 0; i < 20; i += 1) assert.equal((await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: `A${i}` } })).statusCode, 201);
  assert.equal((await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: "one more" } })).statusCode, 400);
});

test("payout: a fresh wallet signature over a message naming the app and the new wallet changes it", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  const app = (await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: "A" } })).json();
  const payout = wallet().publicKey;
  t.chain.setAccount(usdcAccountOf(t.config.usdcMint, payout), accountInfo(TOKEN_PROGRAM, tokenAccountData({ mint: t.config.usdcMint, owner: payout })));

  const challenge = (await t.server.inject({ method: "POST", url: `/v1/apps/${app.id}/payout/challenge`, headers, payload: { payoutWallet: payout.toBase58() } })).json();
  assert.match(challenge.message, new RegExp(app.id));
  assert.match(challenge.message, new RegExp(payout.toBase58()));
  const put = (payload, h = headers) => t.server.inject({ method: "PUT", url: `/v1/apps/${app.id}/payout`, headers: h, payload });

  const wrong = await put({ payoutWallet: payout.toBase58(), nonce: challenge.nonce, signature: sign(Keypair.generate(), challenge.message) });
  assert.equal(wrong.statusCode, 403);
  const elsewhere = await put({ payoutWallet: wallet().publicKey.toBase58(), nonce: challenge.nonce, signature: sign(t.owner, challenge.message) });
  assert.equal(elsewhere.statusCode, 403, "the challenge is for one specific payout wallet");
  const done = await put({ payoutWallet: payout.toBase58(), nonce: challenge.nonce, signature: sign(t.owner, challenge.message) });
  assert.equal(done.statusCode, 200, done.body);
  assert.equal(done.json().payoutWallet, payout.toBase58());
  assert.equal(done.json().ready, true);
  assert.equal((await put({ payoutWallet: payout.toBase58(), nonce: challenge.nonce, signature: sign(t.owner, challenge.message) })).statusCode, 403, "single use");
});

test("payout: the USDC account must exist, and only the owner can ask", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  const app = (await t.server.inject({ method: "POST", url: "/v1/apps", headers, payload: { name: "A" } })).json();
  const payout = wallet().publicKey;
  const challenge = (await t.server.inject({ method: "POST", url: `/v1/apps/${app.id}/payout/challenge`, headers, payload: { payoutWallet: payout.toBase58() } })).json();
  const missing = await t.server.inject({ method: "PUT", url: `/v1/apps/${app.id}/payout`, headers, payload: { payoutWallet: payout.toBase58(), nonce: challenge.nonce, signature: sign(t.owner, challenge.message) } });
  assert.equal(missing.statusCode, 400);
  assert.equal(missing.json().code, "no_payout_account");
  const stranger = await t.server.inject({ method: "POST", url: `/v1/apps/${app.id}/payout/challenge`, headers: t.loginAs(Keypair.generate()), payload: { payoutWallet: payout.toBase58() } });
  assert.equal(stranger.statusCode, 404);
});

test("stats: totals and a daily row for every day of the window", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  const app = await seedApp(t.ctx, { id: "app_stats00001", owner: t.owner.publicKey.toBase58() });
  const today = Date.UTC(2026, 9, 15, 12);
  t.clock.ms = today - 2 * 86_400_000;
  await seedRow(t.ctx.store, { appId: app.id, user: "u", status: "landed", networkCost: 100n, markup: 100n, app: 80n });
  t.clock.ms = today;
  await seedRow(t.ctx.store, { appId: app.id, user: "u", status: "landed", networkCost: 50n, markup: 50n, app: 40n });
  await seedRow(t.ctx.store, { appId: app.id, user: "u", status: "failed" });
  await seedRow(t.ctx.store, { appId: app.id, user: "u", status: "sent" });

  const day = (await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/stats?window=today`, headers })).json();
  assert.deepEqual(day, { window: "today", transactions: 1, failed: 1, volumeMicroUsdc: "100", earnedMicroUsdc: "40", daily: [{ day: "2026-10-15", transactions: 1, failed: 1, earnedMicroUsdc: "40" }] });
  const month = (await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/stats?window=30d`, headers })).json();
  assert.equal(month.daily.length, 30);
  assert.equal(month.daily.at(-1).day, "2026-10-15");
  assert.deepEqual(month.daily.find((d) => d.day === "2026-10-13"), { day: "2026-10-13", transactions: 1, failed: 0, earnedMicroUsdc: "80" });
  assert.equal(month.transactions, 2);
  assert.equal(month.earnedMicroUsdc, "120");
  assert.equal(month.volumeMicroUsdc, "300");
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/stats`, headers })).json().window, "today");
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/stats?window=7d`, headers })).statusCode, 400);
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/stats`, headers: t.loginAs(Keypair.generate()) })).statusCode, 404);
});

test("activity: newest first, statuses mapped, failure reason, paged by cursor", async () => {
  const t = await setup();
  const headers = t.loginAs(t.owner);
  const app = await seedApp(t.ctx, { id: "app_activ00001", owner: t.owner.publicKey.toBase58() });
  const make = async (status, reason) => {
    t.clock.ms += 1000;
    const hash = await seedRow(t.ctx.store, { appId: app.id, user: "u", status, networkCost: 100n, markup: 100n, app: 80n });
    if (reason) await t.ctx.store.recordStatus(hash, { failureReason: reason, signature: "sig" });
  };
  await make("landed");
  await make("failed", "Custom program error");
  await make("expired", "The blockhash expired before the transaction landed");
  await make("sent");
  await make("unknown");
  await make("rejected");

  const list = async (query) => (await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/activity${query}`, headers })).json();
  const page1 = await list("?limit=2");
  assert.deepEqual(page1.items.map((i) => i.status), ["unknown", "pending"]);
  assert.ok(page1.next);
  const page2 = await list(`?limit=2&before=${page1.next}`);
  assert.deepEqual(page2.items.map((i) => i.status), ["expired", "failed"]);
  const page3 = await list(`?limit=2&before=${page2.next}`);
  assert.deepEqual(page3.items.map((i) => i.status), ["landed"]);
  assert.equal(page3.next, null);

  const failed = page2.items[1];
  assert.deepEqual(Object.keys(failed).sort(), ["appMicroUsdc", "createdAt", "failureReason", "networkCostMicroUsdc", "signature", "status", "totalMicroUsdc"]);
  assert.equal(failed.failureReason, "Custom program error");
  assert.equal(failed.totalMicroUsdc, "200");
  assert.equal(failed.appMicroUsdc, "80");
  assert.equal((await list("")).items.length, 5, "the default page holds them all; rejected sends are not listed");

  for (const query of ["?limit=0", "?limit=101", "?limit=x", "?before=garbage"]) {
    assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/activity${query}`, headers })).statusCode, 400, query);
  }
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${app.id}/activity`, headers: t.loginAs(Keypair.generate()) })).statusCode, 404);
});

test("the first transaction time appears on the app after its first send", async () => {
  const t = await setup();
  const headers = t.loginAs({ publicKey: new PublicKey(t.app.owner) });
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${t.app.id}`, headers })).json().firstTransactionAt, null);
  const { signed } = await t.prepareAndSign();
  await t.post("/v1/send", { transaction: signed });
  assert.equal((await t.server.inject({ method: "GET", url: `/v1/apps/${t.app.id}`, headers })).json().firstTransactionAt, new Date(t.clock.ms).toISOString());
});

test("an unclear broadcast followed by the chain settling it: the sweep records landed and the app earns", async () => {
  const t = await setup();
  const { signed, hash } = await t.prepareAndSign();
  t.chain.sendError = new UnknownOutcome("fetch failed");
  const body = (await t.post("/v1/send", { transaction: signed })).json();
  t.chain.statuses.set(body.signature, { err: null });
  await sweepOnce(t.ctx);
  assert.equal((await t.ctx.store.getSend(hash)).status, "landed");
  const headers = t.loginAs({ publicKey: new PublicKey(t.app.owner) });
  const stats = (await t.server.inject({ method: "GET", url: `/v1/apps/${t.app.id}/stats`, headers })).json();
  assert.equal(stats.transactions, 1);
  assert.equal(stats.earnedMicroUsdc, "1200");
});
