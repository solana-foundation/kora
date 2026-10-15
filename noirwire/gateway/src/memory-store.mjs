// The store the tests use. It follows the same contract as the Postgres store (pg-store.mjs).
// Everything between a check and the write that depends on it is synchronous, which is what
// makes reserve() atomic here.

const byOrder = (a, b) => a.createdAt - b.createdAt || (a.messageHash < b.messageHash ? -1 : a.messageHash > b.messageHash ? 1 : 0);

export class MemoryStore {
  constructor({ now = Date.now } = {}) {
    this.now = now;
    this.apps = new Map();
    this.sends = new Map();
  }

  async ping() {}

  async createApp(app) {
    this.apps.set(app.id, { ...app, firstTransactionAt: null, createdAt: this.now() });
  }

  async getApp(id) {
    const app = this.apps.get(id);
    return app ? { ...app } : null;
  }

  async listAppsByOwner(owner) {
    return [...this.apps.values()].filter((app) => app.owner === owner).sort((a, b) => a.createdAt - b.createdAt).map((app) => ({ ...app }));
  }

  async updateApp(id, fields) {
    this.apps.set(id, { ...this.apps.get(id), ...fields });
  }

  #sum(field, filter, sinceMs) {
    let total = 0n;
    for (const row of this.sends.values()) {
      if (row.createdAt >= sinceMs && filter(row)) total += row[field];
    }
    return total;
  }

  /** Lamports the fee payer could lose to open and failed transactions since `sinceMs`: all of them, or one `{ userWallet }`, `{ clientIp }` or `{ appId }`. */
  async lossFor(filter, sinceMs) {
    return this.#sum("lossLamports", (row) => matches(row, filter), sinceMs);
  }

  /** Lamports an app's transactions cost or may cost the fee payer since `sinceMs`. */
  async sponsoredFor(appId, sinceMs) {
    return this.#sum("sponsoredLamports", (row) => row.appId === appId, sinceMs);
  }

  /**
   * Admits a send if it fits every budget, and records it as `sending` with its worst case
   * reserved, in one step. Resolves to `{ status: "reserved" }`, `{ status: "exists" }` when
   * this message already has a row, or `{ status: "refused", code }`.
   */
  async reserve(entry, budgets) {
    if (this.sends.has(entry.messageHash)) return { status: "exists" };
    const since = budgets.sinceMs;
    const refused = (code) => ({ status: "refused", code });
    const fits = (field, filter, limit, amount) => this.#sum(field, filter, since) + amount <= limit;
    if (!fits("lossLamports", () => true, budgets.globalLoss, entry.lossLamports)) return refused("loss_ceiling");
    if (!fits("lossLamports", (row) => row.userWallet === entry.userWallet, budgets.walletLoss, entry.lossLamports)) return refused("wallet_loss_budget");
    if (entry.clientIp && !fits("lossLamports", (row) => row.clientIp === entry.clientIp, budgets.ipLoss, entry.lossLamports)) return refused("ip_loss_budget");
    if (!fits("lossLamports", (row) => row.appId === entry.appId, budgets.appLoss, entry.lossLamports)) return refused("app_loss_budget");
    if (!fits("sponsoredLamports", (row) => row.appId === entry.appId, budgets.appCap, entry.sponsoredLamports)) return refused("app_daily_cap");
    this.sends.set(entry.messageHash, { ...entry, signature: null, signedTransaction: null, failureReason: null, status: "sending", createdAt: this.now() });
    const app = this.apps.get(entry.appId);
    if (app && app.firstTransactionAt === null) app.firstTransactionAt = this.now();
    return { status: "reserved" };
  }

  async getSend(messageHash) {
    const row = this.sends.get(messageHash);
    return row ? { ...row } : null;
  }

  /** Sets whichever of status, signature, signedTransaction, failureReason, lossLamports and sponsoredLamports are given. */
  async recordStatus(messageHash, fields) {
    const row = this.sends.get(messageHash);
    for (const [key, value] of Object.entries(fields)) if (value !== undefined) row[key] = value;
  }

  /** How many of a wallet's sends since `sinceMs` landed and how many failed on chain. Expired and open ones are not counted. */
  async settledCounts(userWallet, sinceMs) {
    const counts = { landed: 0, failed: 0 };
    for (const row of this.sends.values()) {
      if (row.userWallet === userWallet && row.createdAt >= sinceMs && (row.status === "landed" || row.status === "failed")) counts[row.status] += 1;
    }
    return counts;
  }

  /** Takes the broadcast lease on a row if nobody holds a live one. True when this caller now owns it. */
  async claimBroadcast(messageHash, nowMs, leaseMs) {
    const row = this.sends.get(messageHash);
    if (!row || (row.broadcastLeaseUntil ?? 0) > nowMs) return false;
    row.broadcastLeaseUntil = nowMs + leaseMs;
    return true;
  }

  /** A page of `signed`, `sent` and `unknown` rows in creation order, after `cursor` ({ createdAt, messageHash }) when given. */
  async listOpen(cursor, limit) {
    return [...this.sends.values()]
      .filter((row) => ["signed", "sent", "unknown"].includes(row.status) && (!cursor || byOrder(row, cursor) > 0))
      .sort(byOrder)
      .slice(0, limit)
      .map((row) => ({ ...row }));
  }

  /** Turns `sending` rows created before `beforeMs` into `unknown`: the process stopped before Kora answered. */
  async abandonStaleSending(beforeMs) {
    for (const row of this.sends.values()) if (row.status === "sending" && row.createdAt < beforeMs) row.status = "unknown";
  }

  /** Rows still open (sending, signed or sent) that were created before `beforeMs`. */
  async countStale(beforeMs) {
    let count = 0;
    for (const row of this.sends.values()) {
      if (["sending", "signed", "sent"].includes(row.status) && row.createdAt < beforeMs) count += 1;
    }
    return count;
  }

  /** Per UTC day since `sinceMs`: landed and failed counts, landed volume and the app's earnings. */
  async appDaily(appId, sinceMs) {
    const days = new Map();
    for (const row of this.sends.values()) {
      if (row.appId !== appId || row.createdAt < sinceMs) continue;
      const day = new Date(row.createdAt).toISOString().slice(0, 10);
      const entry = days.get(day) ?? { day, landed: 0, failed: 0, volume: 0n, earned: 0n };
      if (row.status === "landed") {
        entry.landed += 1;
        entry.volume += row.networkCost + row.markup;
        entry.earned += row.app;
      } else if (row.status === "failed") {
        entry.failed += 1;
      }
      days.set(day, entry);
    }
    return [...days.values()].sort((a, b) => (a.day < b.day ? -1 : 1));
  }

  /** Newest first, rejected sends left out; `before` is `{ createdAt, messageHash }` from the previous page. */
  async activity(appId, before, limit) {
    return [...this.sends.values()]
      .filter((row) => row.appId === appId && row.status !== "rejected" && (!before || byOrder(row, before) < 0))
      .sort((a, b) => byOrder(b, a))
      .slice(0, limit)
      .map((row) => ({ ...row }));
  }
}

function matches(row, filter) {
  if (filter.userWallet) return row.userWallet === filter.userWallet;
  if (filter.clientIp) return row.clientIp === filter.clientIp;
  if (filter.appId) return row.appId === filter.appId;
  return true;
}
