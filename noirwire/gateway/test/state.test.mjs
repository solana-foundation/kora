import assert from "node:assert/strict";
import { test } from "node:test";
import { Sessions, SESSION_TTL_MS } from "../src/auth.mjs";
import { Challenges, CHALLENGE_TTL_MS } from "../src/challenges.mjs";
import { requireReconciliationHealthy, STALE_AFTER_MS, sweepOnce } from "../src/confirm.mjs";
import { refreshFeePayerHealth, healthView } from "../src/health.mjs";
import { MemoryStore } from "../src/memory-store.mjs";
import { QuoteBook } from "../src/quotes.mjs";
import { RateLimiter } from "../src/ratelimit.mjs";
import { base58Decode, base58Encode, startOfUtcDay } from "../src/units.mjs";
import { FakeChain, seedRow, testConfig } from "./helpers.mjs";

const clockAt = (ms) => ({ ms, now() { return this.ms; } });

test("a quote is single use and expires", () => {
  const clock = clockAt(1000);
  const quotes = new QuoteBook({ now: () => clock.ms });
  quotes.put("h", { expiresAt: 2000 });
  assert.ok(quotes.peek("h"));
  assert.ok(quotes.take("h"));
  assert.equal(quotes.take("h"), null);
  quotes.put("late", { expiresAt: 2000 });
  clock.ms = 2000;
  assert.equal(quotes.peek("late"), null);
  assert.equal(quotes.take("unknown"), null);
});

test("the quote book refuses new work with 503 when full of live quotes, and frees expired ones", () => {
  const clock = clockAt(0);
  const quotes = new QuoteBook({ now: () => clock.ms, maxLive: 2 });
  quotes.put("a", { expiresAt: 100 });
  quotes.put("b", { expiresAt: 100 });
  assert.throws(() => quotes.put("c", { expiresAt: 100 }), (error) => error.code === "overloaded" && error.status === 503);
  clock.ms = 100;
  quotes.put("c", { expiresAt: 200 });
  assert.ok(quotes.peek("c"));
});

test("a challenge is single use, expires, and a bad attempt does not burn it", () => {
  const clock = clockAt(0);
  const challenges = new Challenges({ now: () => clock.ms });
  const build = (nonce) => `text ${nonce}`;
  const { nonce, message } = challenges.issue({ buildMessage: build, meta: { kind: "x" } });
  assert.equal(message, `text ${nonce}`);
  assert.equal(challenges.peek(nonce).meta.kind, "x");
  assert.ok(challenges.peek(nonce), "peeking leaves it");
  assert.equal(challenges.consume(nonce), true);
  assert.equal(challenges.consume(nonce), false);
  const second = challenges.issue({ buildMessage: build, meta: {} });
  clock.ms = CHALLENGE_TTL_MS + 1;
  assert.equal(challenges.consume(second.nonce), false);
  assert.equal(challenges.consume("never issued"), false);
});

test("challenges are capped and answer 503 when full", () => {
  const clock = clockAt(0);
  const challenges = new Challenges({ now: () => clock.ms, max: 2 });
  challenges.issue({ buildMessage: () => "a", meta: {} });
  challenges.issue({ buildMessage: () => "b", meta: {} });
  assert.throws(() => challenges.issue({ buildMessage: () => "c", meta: {} }), { code: "overloaded" });
  clock.ms = CHALLENGE_TTL_MS + 1;
  challenges.issue({ buildMessage: () => "d", meta: {} });
});

test("sessions: random token, hashed at rest, 12 hours, revocable, capped", () => {
  const clock = clockAt(0);
  const sessions = new Sessions({ now: () => clock.ms, max: 2 });
  const { token } = sessions.create("wallet-a");
  assert.equal(sessions.resolve(token), "wallet-a");
  assert.ok(![...sessions.sessions.keys()].includes(token), "the token itself is not stored");
  assert.equal(sessions.resolve("not-a-token"), null);
  sessions.revoke(token);
  assert.equal(sessions.resolve(token), null);

  const b = sessions.create("wallet-b");
  sessions.create("wallet-c");
  assert.throws(() => sessions.create("wallet-d"), { code: "overloaded" });
  clock.ms = SESSION_TTL_MS;
  assert.equal(sessions.resolve(b.token), null, "expired after 12 hours");
  sessions.create("wallet-d");
});

test("the rate limiter counts per key within a window and resets after it", () => {
  const clock = clockAt(0);
  const limiter = new RateLimiter({ limit: 2, windowMs: 60_000, now: () => clock.ms });
  assert.equal(limiter.allow("a"), true);
  assert.equal(limiter.allow("a"), true);
  assert.equal(limiter.allow("a"), false);
  assert.equal(limiter.allow("b"), true);
  clock.ms = 60_000;
  assert.equal(limiter.allow("a"), true);
});

test("the rate limiter caps its keys: 503 when full of live windows, room again after they expire", () => {
  const clock = clockAt(0);
  const limiter = new RateLimiter({ limit: 5, windowMs: 1000, now: () => clock.ms, maxKeys: 3 });
  for (const key of ["a", "b", "c"]) limiter.allow(key);
  assert.throws(() => limiter.allow("d"), (error) => error.code === "overloaded" && error.status === 503);
  assert.equal(limiter.allow("a"), true, "an existing key is still served");
  clock.ms = 1000;
  assert.equal(limiter.allow("d"), true);
  assert.ok(limiter.windows.size <= 3);
});

test("the UTC day starts at midnight", () => {
  const midnight = Date.UTC(2026, 9, 15);
  assert.equal(startOfUtcDay(midnight + 86_399_999), midnight);
  assert.equal(startOfUtcDay(midnight), midnight);
});

test("base58 encodes and decodes", () => {
  assert.equal(base58Encode(Uint8Array.from([0, 0, 1])), "112");
  assert.equal(base58Encode(new Uint8Array(32)), "1".repeat(32));
  assert.deepEqual([...base58Decode("112")], [0, 0, 1]);
  assert.equal(base58Decode("0OIl"), null);
  assert.equal(base58Decode(""), null);
});

const budgets = (overrides = {}) => ({ sinceMs: 0, globalLoss: 1000n, walletLoss: 1000n, ipLoss: 1000n, appLoss: 1000n, appCap: 5000n, ...overrides });
const entry = (hash, overrides = {}) => ({
  messageHash: hash,
  appId: "app",
  userWallet: "user",
  clientIp: "1.1.1.1",
  blockhash: "b",
  networkCostLamports: 1n,
  networkCost: 1n,
  markup: 1n,
  noirwire: 1n,
  app: 0n,
  lossLamports: 100n,
  sponsoredLamports: 300n,
  ...overrides,
});

test("reserve: admits a send once, and reports an existing one", async () => {
  const store = new MemoryStore();
  assert.deepEqual(await store.reserve(entry("a"), budgets()), { status: "reserved" });
  assert.deepEqual(await store.reserve(entry("a"), budgets()), { status: "exists" });
  assert.equal((await store.getSend("a")).status, "sending");
  assert.equal(await store.lossFor({}, 0), 100n);
});

test("reserve: concurrent sends cannot all slip under a limit", async () => {
  const store = new MemoryStore();
  const results = await Promise.all(Array.from({ length: 10 }, (_, i) => store.reserve(entry(`h${i}`, { userWallet: `u${i}`, clientIp: `ip${i}`, appId: "app" }), budgets({ globalLoss: 350n }))));
  assert.equal(results.filter((r) => r.status === "reserved").length, 3);
  assert.ok(results.filter((r) => r.status === "refused").every((r) => r.code === "loss_ceiling"));
});

test("reserve: each budget refuses with its own code, and counts open sends at their worst case", async () => {
  const store = new MemoryStore();
  await store.reserve(entry("a"), budgets());
  const refuse = async (hash, overrides, limits) => (await store.reserve(entry(hash, overrides), budgets(limits))).code;
  assert.equal(await refuse("g", { userWallet: "other", clientIp: "x", appId: "other" }, { globalLoss: 150n }), "loss_ceiling");
  assert.equal(await refuse("w", { clientIp: "x", appId: "other" }, { walletLoss: 150n }), "wallet_loss_budget");
  assert.equal(await refuse("i", { userWallet: "other", appId: "other" }, { ipLoss: 150n }), "ip_loss_budget");
  assert.equal(await refuse("l", { userWallet: "other", clientIp: "x" }, { appLoss: 150n }), "app_loss_budget");
  assert.equal(await refuse("c", { userWallet: "other", clientIp: "x" }, { appCap: 500n }), "app_daily_cap");
  assert.equal(await store.getSend("c"), null, "a refused send leaves no row");
});

test("reserve: the app cap counts everything sponsored, landed or not, and ignores released sends", async () => {
  const store = new MemoryStore();
  await store.reserve(entry("a"), budgets({ appCap: 400n, globalLoss: 10_000n, walletLoss: 10_000n, ipLoss: 10_000n, appLoss: 10_000n }));
  await store.recordStatus("a", { status: "landed", lossLamports: 0n });
  assert.equal(await store.sponsoredFor("app", 0), 300n, "landing keeps the sponsored amount");
  assert.equal((await store.reserve(entry("b"), budgets({ appCap: 400n, globalLoss: 10_000n }))).code, "app_daily_cap");
  await store.recordStatus("a", { status: "expired", lossLamports: 0n, sponsoredLamports: 0n });
  assert.equal((await store.reserve(entry("b"), budgets({ appCap: 400n, globalLoss: 10_000n }))).status, "reserved");
});

test("loss is the reservation while open, the actual fee once failed, nothing once landed", async () => {
  const store = new MemoryStore();
  await store.reserve(entry("a"), budgets());
  assert.equal(await store.lossFor({ userWallet: "user" }, 0), 100n);
  await store.recordStatus("a", { status: "failed", lossLamports: 40n, sponsoredLamports: 40n });
  assert.equal(await store.lossFor({ userWallet: "user" }, 0), 40n);
  await store.reserve(entry("b"), budgets());
  await store.recordStatus("b", { status: "landed", lossLamports: 0n });
  assert.equal(await store.lossFor({ userWallet: "user" }, 0), 40n);
});

test("the first transaction time of an app is recorded once", async () => {
  const clock = clockAt(5000);
  const store = new MemoryStore({ now: () => clock.ms });
  await store.createApp({ id: "app", owner: "o" });
  await store.reserve(entry("a"), budgets());
  clock.ms = 9000;
  await store.reserve(entry("b"), budgets());
  assert.equal((await store.getApp("app")).firstTransactionAt, 5000);
});

async function openRow(store, hash, signature, status = "sent") {
  await store.reserve(entry(hash, { appId: "appx", userWallet: hash }), budgets({ sinceMs: 0, globalLoss: 10n ** 12n, walletLoss: 10n ** 12n, ipLoss: 10n ** 12n, appLoss: 10n ** 12n, appCap: 10n ** 12n }));
  await store.recordStatus(hash, { status, signature });
}

test("confirmation: landed, failed with the fee payer's loss, expired, and still pending", async () => {
  const chain = new FakeChain();
  const clock = clockAt(1_000_000);
  const store = new MemoryStore({ now: () => clock.ms });
  const ctx = { chain, store, now: clock.now.bind(clock) };
  await openRow(store, "landed", "sig-landed");
  await openRow(store, "failed", "sig-failed");
  await openRow(store, "pending", "sig-pending");
  chain.statuses.set("sig-landed", { err: null });
  chain.statuses.set("sig-failed", { err: { InstructionError: [0, "Custom"] } });
  chain.feePayerLoss = 5000n;
  let valid = true;
  chain.isBlockhashValid = async () => valid;

  await sweepOnce(ctx);
  assert.equal((await store.getSend("landed")).status, "landed");
  assert.equal((await store.getSend("landed")).lossLamports, 0n);
  const failed = await store.getSend("failed");
  assert.equal(failed.status, "failed");
  assert.equal(failed.lossLamports, 5000n);
  assert.match(failed.failureReason, /InstructionError/);
  assert.equal((await store.getSend("pending")).status, "sent");

  valid = false;
  await sweepOnce(ctx);
  const expired = await store.getSend("pending");
  assert.equal(expired.status, "expired");
  assert.equal(expired.lossLamports, 0n);
  assert.equal(expired.sponsoredLamports, 0n);
  assert.match(expired.failureReason, /expired/i);
});

test("confirmation pages past rows it cannot settle", async () => {
  const chain = new FakeChain();
  const store = new MemoryStore();
  const ctx = { chain, store, now: () => 1_000_000 };
  for (let i = 0; i < 120; i += 1) await openRow(store, `stuck-${String(i).padStart(3, "0")}`, `sig-stuck-${i}`);
  await openRow(store, "zz-settleable", "sig-ok");
  chain.statuses.set("sig-ok", { err: null });
  await sweepOnce(ctx);
  assert.equal((await store.getSend("zz-settleable")).status, "landed", "the 121st row is reached although 120 older ones stay open");
  assert.equal((await store.getSend("stuck-000")).status, "sent");
});

test("confirmation: a failure whose cost cannot be read yet is retried, and an RPC error does not stop the sweep", async () => {
  const chain = new FakeChain();
  const store = new MemoryStore();
  await openRow(store, "broken", "sig-broken");
  await openRow(store, "unreadable", "sig-unreadable");
  await openRow(store, "fine", "sig-fine");
  chain.statuses.set("sig-unreadable", { err: "x" });
  chain.statuses.set("sig-fine", { err: null });
  chain.feePayerLoss = null;
  const original = chain.getSignatureStatus.bind(chain);
  chain.getSignatureStatus = async (signature) => {
    if (signature === "sig-broken") throw new Error("rpc down");
    return original(signature);
  };
  const ctx = { chain, store, now: () => 1_000_000 };
  await sweepOnce(ctx);
  assert.equal((await store.getSend("broken")).status, "sent");
  assert.equal((await store.getSend("unreadable")).status, "sent");
  assert.equal((await store.getSend("fine")).status, "landed");
  chain.feePayerLoss = 6000n;
  await sweepOnce(ctx);
  assert.equal((await store.getSend("unreadable")).lossLamports, 6000n);
});

test("confirmation: a signed row that never reached a node is rebroadcast with the same bytes, then settled", async () => {
  const chain = new FakeChain();
  const store = new MemoryStore();
  const ctx = { chain, store, now: () => 1_000_000 };
  await openRow(store, "signed-row", "sig-signed", "signed");
  await store.recordStatus("signed-row", { signedTransaction: Buffer.from("the exact signed bytes").toString("base64") });
  await sweepOnce(ctx);
  assert.equal(chain.sent.length, 1);
  assert.equal(chain.sent[0].toString(), "the exact signed bytes");
  assert.equal((await store.getSend("signed-row")).status, "sent");
  await sweepOnce(ctx);
  assert.equal(chain.sent.length, 1, "a sent row is not broadcast again");
});

test("confirmation: stale sending rows become unknown, and are released once their blockhash is gone", async () => {
  const chain = new FakeChain();
  const clock = clockAt(10_000_000);
  const store = new MemoryStore({ now: () => clock.ms });
  const ctx = { chain, store, now: clock.now.bind(clock) };
  await openRow(store, "crashed", null, "sending");
  clock.ms += 6 * 60_000;
  await sweepOnce(ctx);
  const held = await store.getSend("crashed");
  assert.equal(held.status, "unknown");
  assert.equal(held.lossLamports, 100n, "a live blockhash keeps the reservation");

  chain.blockhashValid = false;
  await sweepOnce(ctx);
  const released = await store.getSend("crashed");
  assert.equal(released.status, "expired");
  assert.equal(released.lossLamports, 0n);
  assert.equal(released.sponsoredLamports, 0n);
});

test("confirmation: a signed row whose blockhash is gone and signature absent is released without another broadcast", async () => {
  const chain = new FakeChain();
  const clock = clockAt(10_000_000);
  const store = new MemoryStore({ now: () => clock.ms });
  const ctx = { chain, store, now: clock.now.bind(clock) };
  await openRow(store, "never-sent", "sig-x", "signed");
  await store.recordStatus("never-sent", { signedTransaction: "AAAA" });
  chain.blockhashValid = false;
  await sweepOnce(ctx);
  assert.equal((await store.getSend("never-sent")).status, "expired");
  assert.equal(chain.sent.length, 0);
});

test("reconciliation health: sends are refused only when too many rows are open long past expiry", async () => {
  const clock = clockAt(50_000_000);
  const store = new MemoryStore({ now: () => clock.ms });
  const ctx = { store, now: clock.now.bind(clock), config: { reconciliationStaleLimit: 2 } };
  await requireReconciliationHealthy(ctx);
  for (let i = 0; i < 3; i += 1) await openRow(store, `old-${i}`, `s${i}`);
  await requireReconciliationHealthy(ctx);
  clock.ms += STALE_AFTER_MS + 1;
  await assert.rejects(requireReconciliationHealthy(ctx), (error) => error.code === "reconciliation_unhealthy" && error.status === 503);
  await store.recordStatus("old-0", { status: "landed" });
  await requireReconciliationHealthy(ctx);
});

test("fee payer health: ready, degraded, unavailable, and a failed read keeps the old time", async () => {
  const clock = clockAt(1_000_000);
  const chain = new FakeChain();
  const ctx = { chain, config: testConfig(), now: clock.now.bind(clock) };
  assert.equal(healthView(ctx).state, "unavailable");
  assert.equal(healthView(ctx).checkedAt, new Date(0).toISOString());
  for (const [balance, state] of [[100_000_000n, "ready"], [10_000_000n, "degraded"], [5_000_000n, "unavailable"]]) {
    chain.balance = balance;
    await refreshFeePayerHealth(ctx);
    assert.deepEqual(healthView(ctx), { state, balanceLamports: balance.toString(), checkedAt: new Date(1_000_000).toISOString() });
  }
  chain.balance = 100_000_000n;
  await refreshFeePayerHealth(ctx);
  clock.ms += 120_000;
  chain.getBalance = async () => {
    throw new Error("rpc down");
  };
  await refreshFeePayerHealth(ctx);
  assert.equal(healthView(ctx).state, "unavailable");
  assert.equal(healthView(ctx).checkedAt, new Date(1_000_000).toISOString());
  assert.equal(healthView(ctx).balanceLamports, "100000000");
});

test("activity and daily rows from the store", async () => {
  const clock = clockAt(Date.UTC(2026, 9, 15, 12));
  const store = new MemoryStore({ now: () => clock.ms });
  await store.createApp({ id: "app", owner: "o" });
  await seedRow(store, { appId: "app", user: "u", status: "landed", networkCost: 100n, markup: 100n, app: 80n });
  clock.ms += 86_400_000;
  await seedRow(store, { appId: "app", user: "u", status: "failed" });
  await seedRow(store, { appId: "app", user: "u", status: "rejected" });
  const daily = await store.appDaily("app", 0);
  assert.deepEqual(daily.map((d) => [d.day, d.landed, d.failed, d.volume, d.earned]), [["2026-10-15", 1, 0, 200n, 80n], ["2026-10-16", 0, 1, 0n, 0n]]);
  const rows = await store.activity("app", null, 10);
  assert.equal(rows.length, 2, "rejected sends are not shown");
  assert.equal(rows[0].status, "failed", "newest first");
});
