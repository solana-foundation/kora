import assert from "node:assert/strict";
import { test } from "node:test";
import { Keypair } from "@solana/web3.js";
import { checkAbuseBudgets } from "../src/abuse.mjs";
import { loadConfig } from "../src/config.mjs";
import { makeContext, seedApp, seedRow } from "./helpers.mjs";

async function setup() {
  const made = makeContext();
  const app = await seedApp(made.ctx, { id: "app_budget001" });
  const row = (fields) => seedRow(made.ctx.store, { appId: app.id, ...fields });
  return { ...made, app: await made.ctx.store.getApp(app.id), row };
}

const code = async (promise) => {
  try {
    await promise;
    return null;
  } catch (error) {
    return error.code;
  }
};

test("under the wallet budget passes, at the budget is refused, and a failed row counts its actual fee", async () => {
  const t = await setup();
  await t.row({ user: "w", status: "failed", loss: 49_999n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w", ip: "9.9.9.9", app: t.app })), null);
  await t.row({ user: "w", status: "failed", loss: 1n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w", ip: "9.9.9.9", app: t.app })), "wallet_loss_budget");
});

test("open sends count at their reserved worst case", async () => {
  const t = await setup();
  await t.row({ user: "w", status: "sent", loss: 50_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w", ip: "i", app: t.app })), "wallet_loss_budget");
});

test("one wallet's budget does not block another wallet", async () => {
  const t = await setup();
  await t.row({ user: "bad", ip: "1.1.1.1", status: "failed", loss: 60_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "bad", ip: "2.2.2.2", app: t.app })), "wallet_loss_budget");
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "good", ip: "2.2.2.2", app: t.app })), null);
});

test("the IP budget catches many wallets from one address", async () => {
  const t = await setup();
  t.ctx.config.dailyLossCeilingLamports = 10n ** 9n;
  const other = await seedApp(t.ctx, { id: "app_other0001" });
  for (let i = 0; i < 5; i += 1) await seedRow(t.ctx.store, { appId: other.id, user: `w${i}`, ip: "6.6.6.6", status: "failed", loss: 40_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w0", ip: "7.7.7.7", app: t.app })), null, "each wallet is under its own budget");
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "fresh", ip: "6.6.6.6", app: t.app })), "ip_loss_budget");
});

test("the app loss budget and the app's daily cap are per app", async () => {
  const t = await setup();
  const quiet = await seedApp(t.ctx, { id: "app_quiet0001", dailyCapLamports: 1000n });
  await seedRow(t.ctx.store, { appId: t.app.id, user: "a", ip: "a", status: "failed", loss: 40_000n });
  await seedRow(t.ctx.store, { appId: t.app.id, user: "b", ip: "b", status: "failed", loss: 40_000n });
  await seedRow(t.ctx.store, { appId: t.app.id, user: "c", ip: "c", status: "failed", loss: 20_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "z", ip: "z", app: t.app })), "loss_ceiling", "100,000 is also the global ceiling in this config");
  t.ctx.config.dailyLossCeilingLamports = 10n ** 9n;
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "z", ip: "z", app: t.app })), "app_loss_budget");
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "z", ip: "z", app: await t.ctx.store.getApp(quiet.id) })), null);

  await seedRow(t.ctx.store, { appId: quiet.id, user: "q", ip: "q", status: "landed", sponsored: 1000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "z", ip: "z", app: await t.ctx.store.getApp(quiet.id) })), "app_daily_cap", "landed sends count against the cap too");
});

test("the global ceiling is checked against reserved and failed losses", async () => {
  const t = await setup();
  await t.row({ user: "a", ip: "a", status: "sent", loss: 50_000n });
  await seedRow(t.ctx.store, { appId: t.app.id, user: "b", ip: "b", status: "failed", loss: 50_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "c", ip: "c", app: t.app })), "loss_ceiling");
});

test("loss budgets reset at 00:00 UTC", async () => {
  const t = await setup();
  t.clock.ms = Date.UTC(2026, 9, 15, 23, 59, 0);
  await t.row({ user: "w", status: "failed", loss: 50_000n });
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w", ip: "i", app: t.app })), "wallet_loss_budget");
  t.clock.ms = Date.UTC(2026, 9, 16, 0, 0, 1);
  assert.equal(await code(checkAbuseBudgets(t.ctx, { user: "w", ip: "i", app: t.app })), null);
});

async function withSends(landed, failed, expired = 0) {
  const t = await setup();
  for (let i = 0; i < landed; i += 1) await t.row({ user: "w", ip: `l${i}`, status: "landed" });
  for (let i = 0; i < failed; i += 1) await t.row({ user: "w", ip: `f${i}`, status: "failed", loss: 1n });
  for (let i = 0; i < expired; i += 1) await t.row({ user: "w", ip: `e${i}`, status: "expired" });
  return t;
}

const ratio = async (t) => code(checkAbuseBudgets(t.ctx, { user: "w", ip: "i", app: t.app }));

test("failure ratio: fewer than 5 settled sends is never blocked", async () => {
  assert.equal(await ratio(await withSends(0, 4)), null);
});

test("failure ratio: exactly 20 percent is allowed, above it is blocked", async () => {
  assert.equal(await ratio(await withSends(4, 1)), null, "1 of 5");
  assert.equal(await ratio(await withSends(3, 2)), "wallet_failure_ratio", "2 of 5");
  assert.equal(await ratio(await withSends(8, 2)), null, "2 of 10");
  assert.equal(await ratio(await withSends(7, 3)), "wallet_failure_ratio", "3 of 10");
});

test("failure ratio: expired sends are not counted", async () => {
  assert.equal(await ratio(await withSends(4, 1, 20)), null);
  assert.equal(await ratio(await withSends(1, 3, 10)), null, "only 4 settled, under the minimum");
});

test("failure ratio: only the last 24 hours count", async () => {
  const t = await withSends(0, 5);
  assert.equal(await ratio(t), "wallet_failure_ratio");
  t.clock.ms += 86_400_001;
  assert.equal(await ratio(t), null);
});

test("config: defaults and strict parsing of the abuse limits", () => {
  const env = {
    NETWORK: "devnet",
    RPC_URL: "https://x.example",
    KORA_URL: "http://k",
    KORA_API_KEY: "a",
    KORA_HMAC_SECRET: "b",
    FEE_PAYER: Keypair.generate().publicKey.toBase58(),
    DATABASE_URL: "postgres://x",
  };
  assert.deepEqual(loadConfig(env).abuse, {
    walletDailyLossLamports: 50_000n,
    ipDailyLossLamports: 200_000n,
    appDailyLossLamports: 100_000n,
    failureBlockMinSends: 5,
    failureBlockRatioPpm: 200_000,
  });
  assert.equal(loadConfig({ ...env, FAILURE_BLOCK_RATIO: "0.35", WALLET_DAILY_LOSS_LAMPORTS: "9" }).abuse.failureBlockRatioPpm, 350_000);
  for (const bad of [{ FAILURE_BLOCK_RATIO: "20%" }, { FAILURE_BLOCK_RATIO: "1.5" }, { IP_DAILY_LOSS_LAMPORTS: "0" }, { FAILURE_BLOCK_MIN_SENDS: "x" }, { APP_DAILY_LOSS_LAMPORTS: "0" }]) {
    assert.throws(() => loadConfig({ ...env, ...bad }), /FAILURE_BLOCK|IP_DAILY|MIN_SENDS|APP_DAILY/);
  }
});
