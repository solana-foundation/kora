const CHECK_INTERVAL_MS = 30_000;

/**
 * Reads the fee payer's balance and keeps the result on `ctx.health`. A failed read keeps the
 * last balance and time and marks the state unavailable, so the age of `checkedAt` tells a
 * reader how long the service has been blind.
 */
export async function refreshFeePayerHealth(ctx) {
  const { low, empty } = { low: ctx.config.feePayerThresholds.lowLamports, empty: ctx.config.feePayerThresholds.emptyLamports };
  try {
    const balance = await ctx.chain.getBalance(ctx.config.feePayer);
    const state = balance <= empty ? "unavailable" : balance < low ? "degraded" : "ready";
    ctx.health = { state, balanceLamports: balance, checkedAt: ctx.now() };
  } catch {
    ctx.health = { ...(ctx.health ?? { balanceLamports: 0n, checkedAt: 0 }), state: "unavailable" };
  }
}

export function healthView(ctx) {
  const health = ctx.health ?? { state: "unavailable", balanceLamports: 0n, checkedAt: 0 };
  return {
    state: health.state,
    balanceLamports: health.balanceLamports.toString(),
    checkedAt: new Date(health.checkedAt).toISOString(),
  };
}

export function startHealthChecker(ctx, intervalMs = CHECK_INTERVAL_MS) {
  const timer = setInterval(() => void refreshFeePayerHealth(ctx), intervalMs);
  timer.unref();
  return () => clearInterval(timer);
}
