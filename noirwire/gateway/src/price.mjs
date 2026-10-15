import { PublicKey } from "@solana/web3.js";
import { Refusal } from "./errors.mjs";

/**
 * The reference SOL price, read from Pyth's SOL/USD price account through the same RPC.
 *
 * It is independent of Jupiter: the account is written by Pyth's receiver program, which
 * only accepts a price carrying the Wormhole guardians' signatures. So a quote that is wrong,
 * or a Jupiter that is wrong as a whole, shows up against it. What it is not: proof of the
 * price. An RPC that lies about one account can lie about this one.
 *
 * It is a USD price and users pay in USDC; the two are treated as equal.
 */

// The sponsored SOL/USD feed account (shard 0) and the feed it must carry.
export const PYTH_SOL_USD = new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
const PYTH_RECEIVER = "rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ";
const SOL_USD_FEED_ID = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

/** Older than this and the price is not used. */
export const MAX_PRICE_AGE_SECONDS = 120;
/** The feed's own confidence interval must be within 1 percent of the price. */
const MAX_CONFIDENCE_BPS = 100n;

// PriceUpdateV2: 8 discriminator, 32 write authority, 1 verification level (1 = fully
// verified), then feed id 32, price i64, confidence u64, exponent i32, publish time i64.
const VERIFICATION = 40;
const FEED_ID = 41;
const PRICE = 73;
const CONFIDENCE = 81;
const EXPONENT = 89;
const PUBLISH_TIME = 93;
const MIN_LENGTH = 101;

const refuse = (why) => new Refusal("no_price", `no usable reference SOL price: ${why}`);

/** Raw USDC units per whole SOL, or a Refusal when the price cannot be trusted. */
export async function readReferencePrice(conn, nowSeconds) {
  const account = await conn.getAccountInfo(PYTH_SOL_USD, "confirmed");
  if (!account || account.owner.toBase58() !== PYTH_RECEIVER || account.data.length < MIN_LENGTH) {
    throw refuse("the Pyth account is missing or not Pyth's");
  }
  const data = Buffer.from(account.data);
  if (data[VERIFICATION] !== 1) throw refuse("the price is not fully verified");
  if (data.subarray(FEED_ID, FEED_ID + 32).toString("hex") !== SOL_USD_FEED_ID) {
    throw refuse("the account does not carry the SOL/USD feed");
  }
  const price = data.readBigInt64LE(PRICE);
  const confidence = data.readBigUInt64LE(CONFIDENCE);
  const exponent = data.readInt32LE(EXPONENT);
  const age = nowSeconds - Number(data.readBigInt64LE(PUBLISH_TIME));
  if (age > MAX_PRICE_AGE_SECONDS || age < -MAX_PRICE_AGE_SECONDS) throw refuse(`it is ${age} seconds old`);
  if (price <= 0n || confidence * 10_000n > price * MAX_CONFIDENCE_BPS) throw refuse("its confidence interval is too wide");
  const shift = 6 + exponent; // to micro-dollars
  if (shift < -18 || shift > 18) throw refuse("unexpected exponent");
  return shift >= 0 ? price * 10n ** BigInt(shift) : price / 10n ** BigInt(-shift);
}
