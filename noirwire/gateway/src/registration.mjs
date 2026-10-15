import { randomBytes } from "node:crypto";
import { PublicKey } from "@solana/web3.js";
import { Refusal } from "./errors.mjs";
import { verifyEd25519 } from "./message.mjs";
import { isValidMarkupBps } from "./money.mjs";
import { base58Decode } from "./units.mjs";

const MAX_NAME_LENGTH = 64;
const MAX_ORIGINS = 20;
const MAX_PROGRAMS = 50;
const ID_ALPHABET = "abcdefghijklmnopqrstuvwxyz0123456789";

export function newAppId() {
  const bytes = randomBytes(10);
  return `app_${Array.from(bytes, (byte) => ID_ALPHABET[byte % ID_ALPHABET.length]).join("")}`;
}

export function parseName(value) {
  const name = typeof value === "string" ? value.trim() : "";
  // No control characters: a newline would let a name forge lines of a signed message.
  if (name.length < 1 || name.length > MAX_NAME_LENGTH || /\p{Cc}/u.test(name)) {
    throw new Refusal("bad_request", `name must be 1 to ${MAX_NAME_LENGTH} characters without line breaks`);
  }
  return name;
}

export function parseMarkupBps(value) {
  if (!isValidMarkupBps(value)) throw new Refusal("bad_request", "markupBps must be a whole number from 0 to 30000");
  return value;
}

/** A public key. Signers must be wallets; a payout wallet may be a program-owned address. */
export function parseWallet(value, what, { mustSign = false } = {}) {
  try {
    if (typeof value !== "string") throw new Error("not text");
    const key = new PublicKey(value);
    if (mustSign && !PublicKey.isOnCurve(key.toBytes())) throw new Error("off curve");
    return key;
  } catch {
    throw new Refusal("bad_request", `${what} must be a ${mustSign ? "wallet" : "public key"} address`);
  }
}

export function parseOrigins(value) {
  if (!Array.isArray(value) || value.length > MAX_ORIGINS) throw new Refusal("bad_request", `allowedOrigins must be a list of at most ${MAX_ORIGINS} origins`);
  return value.map((origin) => {
    let parsed = null;
    try {
      parsed = new URL(origin);
    } catch {
      // handled below
    }
    if (!parsed || !["http:", "https:"].includes(parsed.protocol) || parsed.origin !== origin) {
      throw new Refusal("bad_request", "each allowed origin must look like https://example.com, with no path");
    }
    return origin;
  });
}

export function parsePrograms(value) {
  if (!Array.isArray(value) || value.length > MAX_PROGRAMS) throw new Refusal("bad_request", `allowedPrograms must be a list of at most ${MAX_PROGRAMS} program ids`);
  return value.map((program) => parseWallet(program, "each allowed program").toBase58());
}

export function parseDailyCap(value, max) {
  const text = typeof value === "number" ? String(value) : value;
  if (typeof text !== "string" || !/^\d{1,18}$/.test(text) || BigInt(text) < 1n || BigInt(text) > max) {
    throw new Refusal("bad_request", `dailyCapLamports must be a whole number from 1 to ${max}`);
  }
  return BigInt(text);
}

/** The text a wallet signs to sign in. Plain, and says it is not a transaction. */
export function signInMessage({ wallet, network, nonce, issuedAt }) {
  return [
    "noirwire relayer wants you to sign in with your Solana wallet:",
    wallet,
    "",
    "Sign in to manage your apps. This is a message, not a transaction. It costs nothing and moves no funds.",
    "",
    `Network: ${network}`,
    `Nonce: ${nonce}`,
    `Issued At: ${issuedAt}`,
  ].join("\n");
}

/** The text the owner signs to change an app's payout wallet. */
export function payoutMessage({ appId, owner, payoutWallet, network, nonce, issuedAt }) {
  return [
    "noirwire relayer: change payout wallet",
    `App: ${appId}`,
    `Owner wallet: ${owner}`,
    `New payout wallet: ${payoutWallet}`,
    "",
    "Earnings for this app will be paid to the new payout wallet. This is a message, not a transaction. It costs nothing and moves no funds.",
    "",
    `Network: ${network}`,
    `Nonce: ${nonce}`,
    `Issued At: ${issuedAt}`,
  ].join("\n");
}

/** True when `signatureBase58` is `wallet`'s ed25519 signature over the UTF-8 `text`. */
export function walletSignatureValid({ wallet, text, signatureBase58 }) {
  const signature = base58Decode(signatureBase58);
  if (!signature || signature.length !== 64) return false;
  return verifyEd25519(signature, new TextEncoder().encode(text), new PublicKey(wallet).toBytes());
}
