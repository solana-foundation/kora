// Every way a request can end without a signature, kept apart because the caller reacts
// differently to each.

const STATUS = {
  rate_limited: 429,
  wallet_loss_budget: 429,
  ip_loss_budget: 429,
  app_loss_budget: 429,
  app_daily_cap: 429,
  wallet_failure_ratio: 429,
  not_found: 404,
  unknown_app: 404,
  no_quote: 404,
  no_price: 503,
  loss_ceiling: 503,
  unavailable: 503,
  overloaded: 503,
  reconciliation_unhealthy: 503,
  simulation_failed: 422,
  kora_rejected: 422,
  unauthenticated: 401,
  app_settings_changed: 409,
  bad_signature: 403,
  origin_not_allowed: 403,
  app_paused: 403,
  app_not_ready: 403,
  program_not_allowed: 403,
};

/** Nothing was signed or sent. `code` is stable and meant for programs; the message for people. */
export class Refusal extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
    this.status = STATUS[code] ?? 400;
  }
}

/**
 * The relayer or the chain gave an answer that does not say whether the transaction was
 * signed or sent. It may still land, so nothing is retried and no other fee payer is tried.
 */
export class UnknownOutcome extends Error {
  constructor(message, signature = null) {
    super(message);
    this.signature = signature;
  }
}
