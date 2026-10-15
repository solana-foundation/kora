/** What a transaction that was never sent holds in the budgets: nothing. */
export const RELEASED = { lossLamports: 0n, sponsoredLamports: 0n };

/** How long one broadcast attempt owns a row. Longer than the RPC timeout. */
export const BROADCAST_LEASE_MS = 30_000;

export const unknown = (messageHash, signature = null) => ({
  status: 202,
  body: { outcome: "unknown", messageHash, ...(signature ? { signature } : {}) },
});

/** What to tell a caller who sends a message that already has a settled or open row. */
export function recordedOutcome(row) {
  if (row.status === "sending" || row.status === "unknown") return unknown(row.messageHash);
  if (row.status === "rejected") {
    return { status: 422, body: { error: "this transaction was refused earlier and was not sent", code: "rejected" } };
  }
  return { status: 200, body: { signature: row.signature, status: row.status } };
}

/**
 * Broadcasts the exact bytes Kora signed, which are stored on the row; it never signs again.
 *
 * One attempt at a time per message: the caller must win the row's lease, otherwise another
 * attempt is already in flight and this one answers unknown without touching the network.
 * The row becomes `sent` only once a node has accepted the bytes. Any refusal or silence leaves
 * it `signed` with its reservation held: a node's refusal of one attempt proves nothing (an
 * earlier attempt may have landed), so only the sweep settles it, as expired, after the
 * blockhash is gone and the signature is confirmed absent.
 */
export async function broadcastSigned(ctx, row) {
  const { chain, store } = ctx;
  if (!(await store.claimBroadcast(row.messageHash, ctx.now(), BROADCAST_LEASE_MS))) return unknown(row.messageHash, row.signature);
  try {
    await chain.sendRawTransaction(Buffer.from(row.signedTransaction, "base64"));
  } catch {
    await store.recordStatus(row.messageHash, { broadcastLeaseUntil: null });
    return unknown(row.messageHash, row.signature);
  }
  await store.recordStatus(row.messageHash, { status: "sent", broadcastLeaseUntil: null });
  return { status: 200, body: { signature: row.signature, status: "sent" } };
}
