import { Refusal } from "./errors.mjs";
import { DAY_MS, startOfUtcDay } from "./units.mjs";

const PPM = 1_000_000;

/** The limits a send is admitted against, for `reserve`. */
export function reservationBudgets(ctx, app) {
  const { config } = ctx;
  return {
    sinceMs: startOfUtcDay(ctx.now()),
    globalLoss: config.dailyLossCeilingLamports,
    walletLoss: config.abuse.walletDailyLossLamports,
    ipLoss: config.abuse.ipDailyLossLamports,
    appLoss: config.abuse.appDailyLossLamports,
    appCap: app.dailyCapLamports,
  };
}

/**
 * An early, cheap gate in front of the RPC calls: refuses a wallet, IP or app that is already
 * at a limit, and a wallet whose recent sends fail too often. The exact accounting, which
 * includes the send being admitted, is `reserve`. Reads the ledger only.
 *
 * A transaction that passes simulation and then fails on chain costs the fee payer its fee and
 * costs the sender nothing, so one actor must not be able to spend the shared ceiling.
 * Open sends count at their worst case, so concurrent sends cannot all slip under a limit.
 */
export async function checkAbuseBudgets(ctx, { user, ip, app }) {
  const { store, config } = ctx;
  const { abuse } = config;
  const now = ctx.now();
  const today = startOfUtcDay(now);

  if ((await store.lossFor({}, today)) >= config.dailyLossCeilingLamports) {
    throw new Refusal("loss_ceiling", "the relayer has reached its daily loss limit, try again after 00:00 UTC");
  }
  if ((await store.lossFor({ userWallet: user }, today)) >= abuse.walletDailyLossLamports) {
    throw new Refusal("wallet_loss_budget", "this wallet's transactions cost the relayer too much today, try again after 00:00 UTC");
  }
  if (ip && (await store.lossFor({ clientIp: ip }, today)) >= abuse.ipDailyLossLamports) {
    throw new Refusal("ip_loss_budget", "transactions from this address cost the relayer too much today, try again after 00:00 UTC");
  }
  if ((await store.lossFor({ appId: app.id }, today)) >= abuse.appDailyLossLamports) {
    throw new Refusal("app_loss_budget", "this app's transactions cost the relayer too much today, try again after 00:00 UTC");
  }
  if ((await store.sponsoredFor(app.id, today)) >= app.dailyCapLamports) {
    throw new Refusal("app_daily_cap", "this app has reached its daily sponsorship cap, try again after 00:00 UTC");
  }

  const { landed, failed } = await store.settledCounts(user, now - DAY_MS);
  const settled = landed + failed;
  if (settled >= abuse.failureBlockMinSends && failed * PPM > abuse.failureBlockRatioPpm * settled) {
    throw new Refusal("wallet_failure_ratio", "too many of this wallet's recent transactions failed on chain");
  }
}
