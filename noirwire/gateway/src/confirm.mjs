import { broadcastSigned, RELEASED } from "./broadcast.mjs";
import { Refusal } from "./errors.mjs";

const PAGE = 50;
const MAX_ROWS_PER_SWEEP = 5_000;
const ABANDON_SENDING_AFTER_MS = 5 * 60_000;
/** A row still open this long has outlived any blockhash by a wide margin. */
export const STALE_AFTER_MS = 10 * 60_000;
const MAX_REASON_LENGTH = 200;

const describeError = (err) => (typeof err === "string" ? err : JSON.stringify(err)).slice(0, MAX_REASON_LENGTH);

const expire = (ctx, row, reason) => ctx.store.recordStatus(row.messageHash, { status: "expired", failureReason: reason, ...RELEASED });

async function settle(ctx, row) {
  const { chain, store } = ctx;
  if (!row.signature) {
    // Kora's signTransaction signs and returns; it never broadcasts. A row with no signature
    // is one whose answer we never saw, so its transaction can land only if someone holds the
    // signed bytes, and no one can land it once its blockhash is gone. Until then it stays
    // reserved; after that it is released.
    if (await chain.isBlockhashValid(row.blockhash)) return undefined;
    return expire(ctx, row, "The relayer gave no clear answer and the blockhash expired, so the transaction cannot land");
  }
  const status = await chain.getSignatureStatus(row.signature);
  if (status && !status.err) return store.recordStatus(row.messageHash, { status: "landed", lossLamports: 0n });
  if (status) {
    // A failed transaction rolled back the user's payment but still cost the fee payer its fees.
    const lost = await chain.getFeePayerLoss(row.signature);
    if (lost === null) return undefined; // the transaction is not readable yet; look again next sweep
    return store.recordStatus(row.messageHash, {
      status: "failed",
      lossLamports: lost,
      sponsoredLamports: lost,
      failureReason: describeError(status.err),
    });
  }
  if (await chain.isBlockhashValid(row.blockhash)) {
    // Signed and never accepted by a node: send the same bytes again.
    if (row.status === "signed") await broadcastSigned(ctx, row);
    return undefined;
  }
  // The blockhash is gone; look once more in case it landed in the meantime.
  if (await chain.getSignatureStatus(row.signature)) return undefined;
  return expire(ctx, row, "The blockhash expired before the transaction landed");
}

/**
 * One pass over every transaction that was signed or sent and not yet settled. It pages past
 * rows it cannot settle, so a few stuck ones never hide the rest. Never throws.
 */
export async function sweepOnce(ctx) {
  try {
    await ctx.store.abandonStaleSending(ctx.now() - ABANDON_SENDING_AFTER_MS);
  } catch {
    // the database was unreachable; the next sweep tries again
  }
  let cursor = null;
  let seen = 0;
  while (seen < MAX_ROWS_PER_SWEEP) {
    let rows;
    try {
      rows = await ctx.store.listOpen(cursor, PAGE);
    } catch {
      return;
    }
    for (const row of rows) {
      try {
        await settle(ctx, row);
      } catch {
        // The chain was unreachable for this one; it stays open and is looked at again.
      }
    }
    seen += rows.length;
    if (rows.length < PAGE) return;
    cursor = { createdAt: rows.at(-1).createdAt, messageHash: rows.at(-1).messageHash };
  }
}

export function startSweeper(ctx, intervalMs = 5_000) {
  const timer = setInterval(() => void sweepOnce(ctx).catch(() => {}), intervalMs);
  timer.unref();
  return () => clearInterval(timer);
}

/** Sends are refused while too many rows are open long after their blockhash expired: the ledger cannot be trusted. */
export async function requireReconciliationHealthy(ctx) {
  const stale = await ctx.store.countStale(ctx.now() - STALE_AFTER_MS);
  if (stale > ctx.config.reconciliationStaleLimit) {
    throw new Refusal("reconciliation_unhealthy", "the relayer cannot confirm earlier transactions right now, try again later");
  }
}
