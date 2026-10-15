import { Refusal } from "./errors.mjs";

const DEFAULT_MAX_LIVE = 20_000;

/**
 * Quotes by message hash, in memory. A quote is single use and dies at its expiry, so the
 * hash of what the user signed is the only way to a signature: any change to the message
 * finds no quote. The number of live quotes is capped; when it is full, prepare is refused
 * with 503.
 */
export class QuoteBook {
  constructor({ now = Date.now, maxLive = DEFAULT_MAX_LIVE } = {}) {
    this.now = now;
    this.maxLive = maxLive;
    this.quotes = new Map();
  }

  put(hash, quote) {
    if (!this.quotes.has(hash) && this.quotes.size >= this.maxLive) {
      this.prune();
      if (this.quotes.size >= this.maxLive) throw new Refusal("overloaded", "too many open quotes, try again shortly");
    }
    this.quotes.set(hash, quote);
  }

  /** The live quote for `hash`, left in place. */
  peek(hash) {
    const quote = this.quotes.get(hash);
    if (!quote) return null;
    if (quote.expiresAt <= this.now()) {
      this.quotes.delete(hash);
      return null;
    }
    return quote;
  }

  /** The live quote for `hash`, removed so it cannot be used twice. */
  take(hash) {
    const quote = this.peek(hash);
    if (quote) this.quotes.delete(hash);
    return quote;
  }

  prune() {
    const now = this.now();
    for (const [hash, quote] of this.quotes) if (quote.expiresAt <= now) this.quotes.delete(hash);
  }
}
