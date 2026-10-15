import { createHash } from "node:crypto";
import { Refusal } from "./errors.mjs";

/**
 * Per-app switches. The origin list is hygiene, not protection: any script can send any Origin
 * header, and a request without one (a server) is allowed.
 */
export function checkAppAccess(app, origin) {
  if (origin !== undefined && app.allowedOrigins.length > 0 && !app.allowedOrigins.includes(origin)) {
    throw new Refusal("origin_not_allowed", "this app does not accept requests from that origin");
  }
  if (app.paused) throw new Refusal("app_paused", "this app is paused");
}

/** An app on a charging deployment needs somewhere to send its earnings before it can relay. */
export function checkAppReady(app, config) {
  if (config.charge && !app.payoutWallet) throw new Refusal("app_not_ready", "set a payout wallet for this app first");
}

/**
 * What a quote was priced and checked against: the payout wallet, markup and program list.
 * If any of it differs at send, the caller prepares again, so a payout change or a tightened
 * program list never applies to a transaction built under the old settings.
 */
export function settingsFingerprint(app) {
  return createHash("sha256")
    .update(JSON.stringify([app.payoutWallet, app.markupBps, [...app.allowedPrograms].sort()]))
    .digest("hex");
}

export const isReady = (app, config) => !app.paused && (!config.charge || Boolean(app.payoutWallet));
