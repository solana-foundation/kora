const big = (value) => (value === null || value === undefined ? null : BigInt(value));
const ms = (value) => (value ? value.getTime() : null);

// Every reservation takes this advisory lock for its transaction, so the budget sums it reads
// cannot change under it. Reservations are short, and nothing else takes the lock.
const RESERVATION_LOCK = 7_001;

const appFromRow = (row) => ({
  id: row.id,
  owner: row.owner,
  name: row.name,
  payoutWallet: row.payout_wallet,
  markupBps: row.markup_bps,
  allowedOrigins: row.allowed_origins,
  allowedPrograms: row.allowed_programs,
  dailyCapLamports: BigInt(row.daily_cap_lamports),
  paused: row.paused,
  firstTransactionAt: ms(row.first_transaction_at),
  createdAt: ms(row.created_at),
});

const sendFromRow = (row) => ({
  messageHash: row.message_hash,
  appId: row.app_id,
  userWallet: row.user_wallet,
  clientIp: row.client_ip,
  blockhash: row.blockhash,
  signature: row.signature,
  signedTransaction: row.signed_transaction,
  broadcastLeaseUntil: big(row.broadcast_lease_until) === null ? null : Number(row.broadcast_lease_until),
  status: row.status,
  failureReason: row.failure_reason,
  networkCostLamports: big(row.network_cost_lamports),
  networkCost: big(row.network_cost),
  markup: big(row.markup),
  noirwire: big(row.noirwire_share),
  app: big(row.app_share),
  lossLamports: big(row.loss_lamports),
  sponsoredLamports: big(row.sponsored_lamports),
  createdAt: ms(row.created_at),
});

const APP_COLUMNS = {
  name: "name",
  payoutWallet: "payout_wallet",
  markupBps: "markup_bps",
  allowedOrigins: "allowed_origins",
  allowedPrograms: "allowed_programs",
  dailyCapLamports: "daily_cap_lamports",
  paused: "paused",
};

const ROW_COLUMNS = {
  status: "status",
  signature: "signature",
  signedTransaction: "signed_transaction",
  broadcastLeaseUntil: "broadcast_lease_until",
  failureReason: "failure_reason",
  lossLamports: "loss_lamports",
  sponsoredLamports: "sponsored_lamports",
};

const setClause = (columns, fields, firstParam) => {
  const sets = [];
  const params = [];
  for (const [key, value] of Object.entries(fields)) {
    if (value === undefined || !columns[key]) continue;
    params.push(typeof value === "bigint" ? value.toString() : value);
    sets.push(`${columns[key]} = $${firstParam + params.length - 1}`);
  }
  return { sets, params };
};

export class PgStore {
  constructor(pool) {
    this.pool = pool;
  }

  async ping() {
    await this.pool.query("select 1");
  }

  async createApp(app) {
    await this.pool.query(
      `insert into apps (id, owner, name, payout_wallet, markup_bps, allowed_origins, allowed_programs, daily_cap_lamports, paused)
       values ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
      [app.id, app.owner, app.name, app.payoutWallet, app.markupBps, app.allowedOrigins, app.allowedPrograms, app.dailyCapLamports.toString(), app.paused],
    );
  }

  async getApp(id) {
    const { rows } = await this.pool.query("select * from apps where id = $1", [id]);
    return rows[0] ? appFromRow(rows[0]) : null;
  }

  async listAppsByOwner(owner) {
    const { rows } = await this.pool.query("select * from apps where owner = $1 order by created_at", [owner]);
    return rows.map(appFromRow);
  }

  async updateApp(id, fields) {
    const { sets, params } = setClause(APP_COLUMNS, fields, 2);
    if (sets.length === 0) return;
    await this.pool.query(`update apps set ${sets.join(", ")}, updated_at = now() where id = $1`, [id, ...params]);
  }

  async #usage(client, column, filterSql, filterParams, sinceMs) {
    const { rows } = await client.query(
      `select coalesce(sum(${column}), 0) as total from transactions where created_at >= $1 ${filterSql}`,
      [new Date(sinceMs), ...filterParams],
    );
    return BigInt(rows[0].total);
  }

  async lossFor(filter, sinceMs) {
    return this.#filtered("loss_lamports", filter, sinceMs);
  }

  async sponsoredFor(appId, sinceMs) {
    return this.#filtered("sponsored_lamports", { appId }, sinceMs);
  }

  #filtered(column, filter, sinceMs) {
    if (filter.userWallet) return this.#usage(this.pool, column, "and user_wallet = $2", [filter.userWallet], sinceMs);
    if (filter.clientIp) return this.#usage(this.pool, column, "and client_ip = $2", [filter.clientIp], sinceMs);
    if (filter.appId) return this.#usage(this.pool, column, "and app_id = $2", [filter.appId], sinceMs);
    return this.#usage(this.pool, column, "", [], sinceMs);
  }

  async reserve(entry, budgets) {
    const client = await this.pool.connect();
    try {
      await client.query("begin");
      await client.query("select pg_advisory_xact_lock($1)", [RESERVATION_LOCK]);
      const existing = await client.query("select 1 from transactions where message_hash = $1", [entry.messageHash]);
      if (existing.rowCount > 0) {
        await client.query("rollback");
        return { status: "exists" };
      }
      const since = budgets.sinceMs;
      const checks = [
        ["loss_lamports", "", [], budgets.globalLoss, entry.lossLamports, "loss_ceiling"],
        ["loss_lamports", "and user_wallet = $2", [entry.userWallet], budgets.walletLoss, entry.lossLamports, "wallet_loss_budget"],
        ...(entry.clientIp ? [["loss_lamports", "and client_ip = $2", [entry.clientIp], budgets.ipLoss, entry.lossLamports, "ip_loss_budget"]] : []),
        ["loss_lamports", "and app_id = $2", [entry.appId], budgets.appLoss, entry.lossLamports, "app_loss_budget"],
        ["sponsored_lamports", "and app_id = $2", [entry.appId], budgets.appCap, entry.sponsoredLamports, "app_daily_cap"],
      ];
      for (const [column, sql, params, limit, amount, code] of checks) {
        if ((await this.#usage(client, column, sql, params, since)) + amount > limit) {
          await client.query("rollback");
          return { status: "refused", code };
        }
      }
      await client.query(
        `insert into transactions
           (message_hash, app_id, user_wallet, client_ip, blockhash, status, network_cost_lamports, network_cost, markup,
            noirwire_share, app_share, loss_lamports, sponsored_lamports)
         values ($1, $2, $3, $4, $5, 'sending', $6, $7, $8, $9, $10, $11, $12)`,
        [
          entry.messageHash,
          entry.appId,
          entry.userWallet,
          entry.clientIp ?? null,
          entry.blockhash,
          entry.networkCostLamports.toString(),
          entry.networkCost.toString(),
          entry.markup.toString(),
          entry.noirwire.toString(),
          entry.app.toString(),
          entry.lossLamports.toString(),
          entry.sponsoredLamports.toString(),
        ],
      );
      await client.query("update apps set first_transaction_at = now() where id = $1 and first_transaction_at is null", [entry.appId]);
      await client.query("commit");
      return { status: "reserved" };
    } catch (error) {
      await client.query("rollback").catch(() => {});
      throw error;
    } finally {
      client.release();
    }
  }

  async getSend(messageHash) {
    const { rows } = await this.pool.query("select * from transactions where message_hash = $1", [messageHash]);
    return rows[0] ? sendFromRow(rows[0]) : null;
  }

  async recordStatus(messageHash, fields) {
    const { sets, params } = setClause(ROW_COLUMNS, fields, 2);
    if (sets.length === 0) return;
    await this.pool.query(`update transactions set ${sets.join(", ")}, updated_at = now() where message_hash = $1`, [messageHash, ...params]);
  }

  /** Takes the broadcast lease on a row if nobody holds a live one. True when this caller now owns it. */
  async claimBroadcast(messageHash, nowMs, leaseMs) {
    const { rowCount } = await this.pool.query(
      `update transactions set broadcast_lease_until = $3
        where message_hash = $1 and (broadcast_lease_until is null or broadcast_lease_until <= $2)`,
      [messageHash, String(nowMs), String(nowMs + leaseMs)],
    );
    return rowCount === 1;
  }

  async settledCounts(userWallet, sinceMs) {
    const { rows } = await this.pool.query(
      `select status, count(*) as n from transactions
        where user_wallet = $1 and created_at >= $2 and status in ('landed', 'failed') group by status`,
      [userWallet, new Date(sinceMs)],
    );
    const counts = { landed: 0, failed: 0 };
    for (const row of rows) counts[row.status] = Number(row.n);
    return counts;
  }

  async listOpen(cursor, limit) {
    const { rows } = cursor
      ? await this.pool.query(
          `select * from transactions where status in ('signed', 'sent', 'unknown') and (created_at, message_hash) > ($1, $2)
            order by created_at, message_hash limit $3`,
          [new Date(cursor.createdAt), cursor.messageHash, limit],
        )
      : await this.pool.query("select * from transactions where status in ('signed', 'sent', 'unknown') order by created_at, message_hash limit $1", [limit]);
    return rows.map(sendFromRow);
  }

  async abandonStaleSending(beforeMs) {
    await this.pool.query("update transactions set status = 'unknown', updated_at = now() where status = 'sending' and created_at < $1", [new Date(beforeMs)]);
  }

  async countStale(beforeMs) {
    const { rows } = await this.pool.query(
      "select count(*) as n from transactions where status in ('sending', 'signed', 'sent') and created_at < $1",
      [new Date(beforeMs)],
    );
    return Number(rows[0].n);
  }

  async appDaily(appId, sinceMs) {
    const { rows } = await this.pool.query(
      `select to_char(created_at at time zone 'UTC', 'YYYY-MM-DD') as day,
              count(*) filter (where status = 'landed') as landed,
              count(*) filter (where status = 'failed') as failed,
              coalesce(sum(network_cost + markup) filter (where status = 'landed'), 0) as volume,
              coalesce(sum(app_share) filter (where status = 'landed'), 0) as earned
         from transactions where app_id = $1 and created_at >= $2 group by 1 order by 1`,
      [appId, new Date(sinceMs)],
    );
    return rows.map((row) => ({ day: row.day, landed: Number(row.landed), failed: Number(row.failed), volume: BigInt(row.volume), earned: BigInt(row.earned) }));
  }

  async activity(appId, before, limit) {
    const { rows } = before
      ? await this.pool.query(
          `select * from transactions where app_id = $1 and status <> 'rejected' and (created_at, message_hash) < ($2, $3)
            order by created_at desc, message_hash desc limit $4`,
          [appId, new Date(before.createdAt), before.messageHash, limit],
        )
      : await this.pool.query("select * from transactions where app_id = $1 and status <> 'rejected' order by created_at desc, message_hash desc limit $2", [appId, limit]);
    return rows.map(sendFromRow);
  }
}
