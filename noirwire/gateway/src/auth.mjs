import { createHash, randomBytes } from "node:crypto";
import { Refusal } from "./errors.mjs";
import { parseWallet, signInMessage, walletSignatureValid } from "./registration.mjs";

export const SESSION_TTL_MS = 12 * 3_600_000;
const DEFAULT_MAX_SESSIONS = 20_000;

const hashOf = (token) => createHash("sha256").update(token).digest("hex");

/** Bearer sessions in memory. Only the hash of a token is kept. Capped; 503 when full. */
export class Sessions {
  constructor({ now = Date.now, max = DEFAULT_MAX_SESSIONS } = {}) {
    this.now = now;
    this.max = max;
    this.sessions = new Map();
  }

  create(wallet) {
    if (this.sessions.size >= this.max) this.prune();
    if (this.sessions.size >= this.max) throw new Refusal("overloaded", "too many open sessions, try again shortly");
    const token = randomBytes(32).toString("base64url");
    const expiresAt = this.now() + SESSION_TTL_MS;
    this.sessions.set(hashOf(token), { wallet, expiresAt });
    return { token, expiresAt };
  }

  /** The wallet a token belongs to, or null. */
  resolve(token) {
    const session = this.sessions.get(hashOf(token));
    if (!session) return null;
    if (session.expiresAt <= this.now()) {
      this.sessions.delete(hashOf(token));
      return null;
    }
    return session.wallet;
  }

  revoke(token) {
    this.sessions.delete(hashOf(token));
  }

  prune() {
    const now = this.now();
    for (const [key, session] of this.sessions) if (session.expiresAt <= now) this.sessions.delete(key);
  }
}

export const bearerToken = (request) => /^Bearer (\S+)$/.exec(request.headers.authorization ?? "")?.[1] ?? null;

/** The wallet behind the request's bearer token, or a 401. */
export function authenticate(ctx, request) {
  const token = bearerToken(request);
  const wallet = token ? ctx.sessions.resolve(token) : null;
  if (!wallet) throw new Refusal("unauthenticated", "sign in first");
  return wallet;
}

export function issueSignInChallenge(ctx, body) {
  const wallet = parseWallet(body.wallet, "wallet", { mustSign: true }).toBase58();
  const { network } = ctx.config;
  const { nonce, message, expiresAt } = ctx.challenges.issue({
    meta: { kind: "signin", wallet },
    buildMessage: (nonce, issuedAt) => signInMessage({ wallet, network, nonce, issuedAt }),
  });
  return { nonce, message, expiresAt: new Date(expiresAt).toISOString() };
}

/** Checks the signature on the issued text, burns the nonce, and opens a session for the wallet. */
export function openSession(ctx, body) {
  const wallet = parseWallet(body.wallet, "wallet", { mustSign: true }).toBase58();
  const challenge = typeof body.nonce === "string" ? ctx.challenges.peek(body.nonce) : null;
  if (!challenge || challenge.meta.kind !== "signin" || challenge.meta.wallet !== wallet) {
    throw new Refusal("bad_signature", "the challenge is unknown, already used or expired");
  }
  if (!walletSignatureValid({ wallet, text: challenge.message, signatureBase58: body.signature })) {
    throw new Refusal("bad_signature", "the signature does not match the sign-in message");
  }
  if (!ctx.challenges.consume(body.nonce)) throw new Refusal("bad_signature", "the challenge is unknown, already used or expired");
  const { token, expiresAt } = ctx.sessions.create(wallet);
  return { token, expiresAt: new Date(expiresAt).toISOString() };
}
