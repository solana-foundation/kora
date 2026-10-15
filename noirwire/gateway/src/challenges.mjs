import { randomBytes } from "node:crypto";
import { Refusal } from "./errors.mjs";

export const CHALLENGE_TTL_MS = 5 * 60_000;
const DEFAULT_MAX_OUTSTANDING = 20_000;

/**
 * Single use challenges in memory: a nonce, the exact text the wallet must sign, and what the
 * challenge is for. The number outstanding is capped; when it is full, new ones get 503.
 */
export class Challenges {
  constructor({ now = Date.now, max = DEFAULT_MAX_OUTSTANDING } = {}) {
    this.now = now;
    this.max = max;
    this.entries = new Map();
  }

  /** `buildMessage(nonce, issuedAtIso)` returns the text to sign; `meta` is kept with it. */
  issue({ buildMessage, meta }) {
    if (this.entries.size >= this.max) this.prune();
    if (this.entries.size >= this.max) throw new Refusal("overloaded", "too many open challenges, try again shortly");
    const nonce = randomBytes(16).toString("hex");
    const issuedAt = this.now();
    const expiresAt = issuedAt + CHALLENGE_TTL_MS;
    const message = buildMessage(nonce, new Date(issuedAt).toISOString());
    this.entries.set(nonce, { message, meta, expiresAt });
    return { nonce, message, expiresAt };
  }

  /** The live challenge, left in place so a bad signature does not burn it. */
  peek(nonce) {
    const entry = this.entries.get(nonce);
    return entry && entry.expiresAt > this.now() ? entry : null;
  }

  /** True once per live nonce. */
  consume(nonce) {
    const entry = this.peek(nonce);
    this.entries.delete(nonce);
    return entry !== null;
  }

  prune() {
    const now = this.now();
    for (const [nonce, entry] of this.entries) if (entry.expiresAt <= now) this.entries.delete(nonce);
  }
}
