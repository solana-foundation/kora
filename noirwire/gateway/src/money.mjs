import { Refusal } from "./errors.mjs";
import { ceilDiv, LAMPORTS_PER_SOL } from "./units.mjs";

export const BPS = 10_000n;
/** The markup a new app starts with: the network cost again, so the user pays 2x. */
export const DEFAULT_MARKUP_BPS = 10_000;
export const MAX_MARKUP_BPS = 30_000;
/** The app receives 4/5 of the markup; NoirWire keeps ceil(1/5). */
export const NOIRWIRE_SHARE_DIVISOR = 5n;

/** Lamports to micro-USDC at `microUsdcPerSol`, rounded up so the relayer never under-collects. */
export function lamportsToMicroUsdc(lamports, microUsdcPerSol) {
  if (lamports < 0n || microUsdcPerSol <= 0n) throw new Refusal("no_price", "cannot convert without a positive price");
  return ceilDiv(lamports * microUsdcPerSol, LAMPORTS_PER_SOL);
}

export const isValidMarkupBps = (value) => Number.isInteger(value) && value >= 0 && value <= MAX_MARKUP_BPS;

/**
 * Splits what the user pays.
 *   networkCost B (micro-USDC), markup M = ceil(B * m / 10000)
 *   NoirWire gets S = ceil(M / 5), the app gets C = M - S
 */
export function splitPayment({ networkCost, markupBps }) {
  if (networkCost < 0n) throw new RangeError("networkCost must not be negative");
  if (!isValidMarkupBps(markupBps)) throw new RangeError("markupBps out of range");
  const markup = ceilDiv(networkCost * BigInt(markupBps), BPS);
  const noirwire = ceilDiv(markup, NOIRWIRE_SHARE_DIVISOR);
  const app = markup - noirwire;
  return { networkCost, markup, noirwire, app, total: networkCost + markup };
}
