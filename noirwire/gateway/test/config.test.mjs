import assert from "node:assert/strict";
import { test } from "node:test";
import { Keypair, PublicKey } from "@solana/web3.js";
import { ConfigError, loadConfig } from "../src/config.mjs";

const feePayer = Keypair.generate().publicKey.toBase58();
const paymentAccount = Keypair.generate().publicKey.toBase58();

const devnet = () => ({
  NETWORK: "devnet",
  RPC_URL: "https://api.devnet.solana.com",
  KORA_URL: "http://kora.internal:8080",
  KORA_API_KEY: "key",
  KORA_HMAC_SECRET: "secret",
  FEE_PAYER: feePayer,
  DATABASE_URL: "postgres://user:pw@db/gateway",
});

const mainnet = () => ({ ...devnet(), NETWORK: "mainnet", PAYMENT_USDC_ACCOUNT: paymentAccount });

const problems = (env) => {
  try {
    loadConfig(env);
  } catch (error) {
    assert.ok(error instanceof ConfigError);
    return error.message;
  }
  return null;
};

test("devnet defaults: charging off, no payment account needed, default devnet USDC", () => {
  const config = loadConfig(devnet());
  assert.equal(config.charge, false);
  assert.equal(config.network, "devnet");
  assert.equal(config.paymentUsdcAccount, null);
  assert.equal(config.usdcMint.toBase58(), "4zMMC9srt5Ri5X14GAgXhaHii3GfPDcTjbL1bShxjvrZ");
  assert.equal(config.port, 8080);
  assert.equal(config.dailyLossCeilingLamports, 100_000_000n);
  assert.equal(config.caps.maxTxBytes, 1232);
  assert.equal(config.caps.maxComputeUnitPrice, 0, "no priority fee unless the operator allows one");
  assert.equal(config.caps.maxPriorityFeeLamports, 0);
  assert.equal(loadConfig({ ...devnet(), MAX_PRIORITY_FEE_LAMPORTS: "10000" }).caps.maxPriorityFeeLamports, 10_000);
  assert.match(problems({ ...devnet(), MAX_PRIORITY_FEE_LAMPORTS: "10001" }), /MAX_PRIORITY_FEE_LAMPORTS/);
  assert.equal(config.trustedProxyHops, 0);
  assert.equal(config.apps.dailyCapDefaultLamports, 10_000_000n);
  assert.equal(config.abuse.appDailyLossLamports, 100_000n);
  assert.equal(config.reconciliationStaleLimit, 20);
  assert.equal(config.limits.globalPerMinute, 3000);
});

test("mainnet defaults: charging on with the real USDC mint", () => {
  const config = loadConfig(mainnet());
  assert.equal(config.charge, true);
  assert.equal(config.usdcMint.toBase58(), "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  assert.equal(config.paymentUsdcAccount.toBase58(), paymentAccount);
});

test("mainnet refuses to start with charging off", () => {
  assert.match(problems({ ...mainnet(), CHARGE: "off" }), /CHARGE: mainnet cannot run with charging off/);
});

test("charging on requires the payment account", () => {
  assert.match(problems({ ...devnet(), CHARGE: "on" }), /PAYMENT_USDC_ACCOUNT/);
});

test("required settings are named when missing", () => {
  const msg = problems({});
  for (const name of ["NETWORK", "RPC_URL", "KORA_URL", "KORA_API_KEY", "KORA_HMAC_SECRET", "FEE_PAYER", "DATABASE_URL"]) {
    assert.match(msg, new RegExp(name));
  }
});

test("values are checked strictly and never echoed", () => {
  const secret = "not-a-url-but-secret-looking-value";
  const msg = problems({ ...devnet(), RPC_URL: secret, MAX_SIGNATURES: "many", QUOTE_TTL_SECONDS: "9999", CHARGE: "maybe", TRUSTED_PROXY_HOPS: "2" });
  assert.match(msg, /RPC_URL/);
  assert.match(msg, /MAX_SIGNATURES/);
  assert.match(msg, /QUOTE_TTL_SECONDS/);
  assert.match(msg, /CHARGE/);
  assert.match(msg, /TRUSTED_PROXY_HOPS/);
  assert.ok(!msg.includes(secret));
});

test("the fee payer must be a wallet, not an off-curve address", () => {
  const [derived] = PublicKey.findProgramAddressSync([Buffer.from("seed")], Keypair.generate().publicKey);
  const offCurve = derived.toBase58();
  assert.match(problems({ ...devnet(), FEE_PAYER: offCurve }), /FEE_PAYER: not a wallet address/);
});

test("ceilings cannot be exceeded", () => {
  assert.match(problems({ ...devnet(), MAX_TX_BYTES: "5000" }), /MAX_TX_BYTES/);
  assert.match(problems({ ...devnet(), MAX_COMPUTE_UNIT_LIMIT: "2000000" }), /MAX_COMPUTE_UNIT_LIMIT/);
  assert.match(problems({ ...devnet(), MAX_FEE_PAYER_OUTFLOW_LAMPORTS: "999999999" }), /MAX_FEE_PAYER_OUTFLOW_LAMPORTS/);
  assert.match(problems({ ...devnet(), DAILY_LOSS_CEILING_LAMPORTS: "0" }), /DAILY_LOSS_CEILING_LAMPORTS/);
});

test("overrides are read", () => {
  const config = loadConfig({ ...devnet(), CHARGE: "on", PAYMENT_USDC_ACCOUNT: paymentAccount, PORT: "9000", RATE_LIMIT_IP_PER_MINUTE: "5", TRUSTED_PROXY_HOPS: "1", QUOTE_TTL_SECONDS: "30" });
  assert.equal(config.charge, true);
  assert.equal(config.port, 9000);
  assert.equal(config.limits.ipPerMinute, 5);
  assert.equal(config.trustedProxyHops, 1);
  assert.equal(config.quoteTtlSeconds, 30);
});

test("app caps, fee payer thresholds and proxy hops are checked", () => {
  assert.match(problems({ ...devnet(), APP_DAILY_CAP_DEFAULT_LAMPORTS: "900", APP_DAILY_CAP_MAX_LAMPORTS: "100" }), /APP_DAILY_CAP_DEFAULT_LAMPORTS: must not exceed/);
  assert.match(problems({ ...devnet(), FEE_PAYER_EMPTY_LAMPORTS: "30000000" }), /FEE_PAYER_EMPTY_LAMPORTS: must be below/);
  assert.match(problems({ ...devnet(), TRUSTED_PROXY_HOPS: "-1" }), /TRUSTED_PROXY_HOPS/);
});
