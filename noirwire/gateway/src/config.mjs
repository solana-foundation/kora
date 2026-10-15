import { PublicKey } from "@solana/web3.js";
import { MAX_COMPUTE_UNITS } from "./policy.mjs";

// Everything comes from the environment, is checked once here, and is refused as a whole if
// any part of it makes no sense. A service that moves money should not start on a typo.

export class ConfigError extends Error {}

const USDC_MINTS = {
  mainnet: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  devnet: "4zMMC9srt5Ri5X14GAgXhaHii3GfPDcTjbL1bShxjvrZ",
};

const DEFAULTS = {
  PORT: "8080",
  MAX_SIGNATURES: "3",
  MAX_TX_BYTES: "1232",
  MAX_COMPUTE_UNIT_PRICE: "0",
  MAX_COMPUTE_UNIT_LIMIT: String(MAX_COMPUTE_UNITS),
  MAX_FEE_PAYER_OUTFLOW_LAMPORTS: "2200000",
  MAX_PRIORITY_FEE_LAMPORTS: "0",
  RATE_LIMIT_GLOBAL_PER_MINUTE: "3000",
  RATE_LIMIT_IP_PER_MINUTE: "60",
  RATE_LIMIT_USER_PER_MINUTE: "20",
  RATE_LIMIT_REGISTER_PER_MINUTE: "10",
  QUOTE_TTL_SECONDS: "60",
  DAILY_LOSS_CEILING_LAMPORTS: "100000000",
  TRUSTED_PROXY_HOPS: "0",
  WALLET_DAILY_LOSS_LAMPORTS: "50000",
  IP_DAILY_LOSS_LAMPORTS: "200000",
  APP_DAILY_LOSS_LAMPORTS: "100000",
  APP_DAILY_CAP_DEFAULT_LAMPORTS: "10000000",
  APP_DAILY_CAP_MAX_LAMPORTS: "500000000",
  FAILURE_BLOCK_MIN_SENDS: "5",
  FAILURE_BLOCK_RATIO: "0.2",
  RECONCILIATION_STALE_LIMIT: "20",
  FEE_PAYER_LOW_LAMPORTS: "30000000",
  FEE_PAYER_EMPTY_LAMPORTS: "5000000",
};

// Ceilings no setting may exceed.
const HARD_MAX_TX_BYTES = 1232; // one packet
const HARD_MAX_SIGNATURES = 8;
const HARD_MAX_OUTFLOW_LAMPORTS = 10_000_000;
const HARD_MAX_PRIORITY_FEE_LAMPORTS = 10_000; // the total per transaction, whatever price and limit say
const HARD_MAX_QUOTE_TTL_SECONDS = 90; // a blockhash lives about 60 to 90 seconds
const SAFE_MAX = Number.MAX_SAFE_INTEGER;

export function loadConfig(env) {
  const problems = [];
  const read = (name) => {
    const value = env[name]?.trim();
    return value ? value : DEFAULTS[name];
  };
  const integer = (name, min, max) => {
    const value = read(name);
    if (!/^\d+$/.test(value) || Number(value) < min || Number(value) > max) {
      problems.push(`${name}: must be a whole number from ${min} to ${max}`);
      return min;
    }
    return Number(value);
  };
  const lamports = (name, min = 1, max = SAFE_MAX) => BigInt(integer(name, min, max));
  const publicKey = (name, value, { onCurve = false } = {}) => {
    try {
      const key = new PublicKey(value);
      if (onCurve && !PublicKey.isOnCurve(key.toBytes())) problems.push(`${name}: not a wallet address (off curve)`);
      return key;
    } catch {
      problems.push(`${name}: missing or not a public key`);
      return null;
    }
  };
  const url = (name) => {
    const value = read(name);
    if (!value || !/^https?:\/\//.test(value)) problems.push(`${name}: must be an http(s) URL`);
    return value;
  };

  const network = read("NETWORK");
  if (network !== "devnet" && network !== "mainnet") problems.push('NETWORK: must be "devnet" or "mainnet"');

  const chargeText = read("CHARGE")?.toLowerCase() ?? (network === "mainnet" ? "on" : "off");
  if (!["on", "off"].includes(chargeText)) problems.push('CHARGE: must be "on" or "off"');
  const charge = chargeText === "on";
  if (network === "mainnet" && !charge) problems.push("CHARGE: mainnet cannot run with charging off");

  const rpcUrl = url("RPC_URL");
  const koraUrl = url("KORA_URL");
  const koraApiKey = read("KORA_API_KEY");
  const koraHmacSecret = read("KORA_HMAC_SECRET");
  if (!koraApiKey) problems.push("KORA_API_KEY: required");
  if (!koraHmacSecret) problems.push("KORA_HMAC_SECRET: required");

  const feePayer = publicKey("FEE_PAYER", read("FEE_PAYER") ?? "", { onCurve: true });
  const usdcMint = publicKey("USDC_MINT", read("USDC_MINT") ?? USDC_MINTS[network] ?? "");
  const paymentUsdcAccount = charge ? publicKey("PAYMENT_USDC_ACCOUNT", read("PAYMENT_USDC_ACCOUNT") ?? "") : null;
  if (!read("DATABASE_URL")) problems.push("DATABASE_URL: required");

  const ratioText = read("FAILURE_BLOCK_RATIO");
  if (!/^(0(\.\d{1,6})?|1(\.0{1,6})?)$/.test(ratioText)) problems.push("FAILURE_BLOCK_RATIO: must be a decimal from 0 to 1");

  const dailyCapDefault = lamports("APP_DAILY_CAP_DEFAULT_LAMPORTS");
  const dailyCapMax = lamports("APP_DAILY_CAP_MAX_LAMPORTS");
  if (dailyCapDefault > dailyCapMax) problems.push("APP_DAILY_CAP_DEFAULT_LAMPORTS: must not exceed APP_DAILY_CAP_MAX_LAMPORTS");
  const feePayerLow = lamports("FEE_PAYER_LOW_LAMPORTS");
  const feePayerEmpty = lamports("FEE_PAYER_EMPTY_LAMPORTS");
  if (feePayerEmpty >= feePayerLow) problems.push("FEE_PAYER_EMPTY_LAMPORTS: must be below FEE_PAYER_LOW_LAMPORTS");

  const config = {
    network,
    charge,
    port: integer("PORT", 1, 65535),
    rpcUrl,
    koraUrl,
    koraApiKey,
    koraHmacSecret,
    databaseUrl: read("DATABASE_URL"),
    trustedProxyHops: integer("TRUSTED_PROXY_HOPS", 0, 1),
    quoteTtlSeconds: integer("QUOTE_TTL_SECONDS", 5, HARD_MAX_QUOTE_TTL_SECONDS),
    dailyLossCeilingLamports: lamports("DAILY_LOSS_CEILING_LAMPORTS"),
    caps: {
      maxSignatures: integer("MAX_SIGNATURES", 2, HARD_MAX_SIGNATURES),
      maxTxBytes: integer("MAX_TX_BYTES", 200, HARD_MAX_TX_BYTES),
      maxComputeUnitPrice: integer("MAX_COMPUTE_UNIT_PRICE", 0, 10_000_000),
      maxComputeUnitLimit: integer("MAX_COMPUTE_UNIT_LIMIT", 1, MAX_COMPUTE_UNITS),
      maxPriorityFeeLamports: integer("MAX_PRIORITY_FEE_LAMPORTS", 0, HARD_MAX_PRIORITY_FEE_LAMPORTS),
      maxFeePayerOutflowLamports: integer("MAX_FEE_PAYER_OUTFLOW_LAMPORTS", 0, HARD_MAX_OUTFLOW_LAMPORTS),
    },
    limits: {
      globalPerMinute: integer("RATE_LIMIT_GLOBAL_PER_MINUTE", 1, 10_000_000),
      ipPerMinute: integer("RATE_LIMIT_IP_PER_MINUTE", 1, 100_000),
      userPerMinute: integer("RATE_LIMIT_USER_PER_MINUTE", 1, 100_000),
      registerPerMinute: integer("RATE_LIMIT_REGISTER_PER_MINUTE", 1, 100_000),
    },
    abuse: {
      walletDailyLossLamports: lamports("WALLET_DAILY_LOSS_LAMPORTS"),
      ipDailyLossLamports: lamports("IP_DAILY_LOSS_LAMPORTS"),
      appDailyLossLamports: lamports("APP_DAILY_LOSS_LAMPORTS"),
      failureBlockMinSends: integer("FAILURE_BLOCK_MIN_SENDS", 1, 10_000),
      failureBlockRatioPpm: Math.round(Number(ratioText) * 1_000_000),
    },
    apps: { dailyCapDefaultLamports: dailyCapDefault, dailyCapMaxLamports: dailyCapMax },
    reconciliationStaleLimit: integer("RECONCILIATION_STALE_LIMIT", 0, 1_000_000),
    feePayerThresholds: { lowLamports: feePayerLow, emptyLamports: feePayerEmpty },
    feePayer,
    usdcMint,
    paymentUsdcAccount,
  };

  if (problems.length > 0) throw new ConfigError(problems.join("; "));
  return config;
}
