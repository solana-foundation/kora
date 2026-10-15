import { Refusal } from "./errors.mjs";

const DEFAULT_MAX_KEYS = 100_000;

/**
 * A fixed window counter per key, in memory. One instance per limit. The number of live keys
 * is capped: when it is full of unexpired windows a new key is refused with 503 rather than
 * letting memory grow.
 */
export class RateLimiter {
  constructor({ limit, windowMs = 60_000, now = Date.now, maxKeys = DEFAULT_MAX_KEYS }) {
    this.limit = limit;
    this.windowMs = windowMs;
    this.now = now;
    this.maxKeys = maxKeys;
    this.windows = new Map();
  }

  /** Counts one request for `key`; false once the key is over its limit for this window. */
  allow(key) {
    const now = this.now();
    const current = this.windows.get(key);
    if (!current || now - current.start >= this.windowMs) {
      if (!current && this.windows.size >= this.maxKeys) {
        this.prune(now);
        if (this.windows.size >= this.maxKeys) throw new Refusal("overloaded", "the service is busy, try again shortly");
      }
      this.windows.set(key, { start: now, count: 1 });
      return true;
    }
    current.count += 1;
    return current.count <= this.limit;
  }

  prune(now) {
    for (const [key, window] of this.windows) {
      if (now - window.start >= this.windowMs) this.windows.delete(key);
    }
  }
}

/** Every limiter the gateway uses, built once. */
export function createLimiters(config, now = Date.now) {
  return {
    global: new RateLimiter({ limit: config.limits.globalPerMinute, now, maxKeys: 1 }),
    ip: new RateLimiter({ limit: config.limits.ipPerMinute, now }),
    user: new RateLimiter({ limit: config.limits.userPerMinute, now }),
    registration: new RateLimiter({ limit: config.limits.registerPerMinute, now }),
  };
}
