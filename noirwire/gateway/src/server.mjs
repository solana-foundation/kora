import cors from "@fastify/cors";
import Fastify from "fastify";
import { appActivity, appStats, createApp, getApp, issuePayoutChallenge, listApps, patchApp, setPayout } from "./apps.mjs";
import { Sessions, authenticate, bearerToken, issueSignInChallenge, openSession } from "./auth.mjs";
import { Refusal } from "./errors.mjs";
import { healthView } from "./health.mjs";
import { decodeTransaction } from "./message.mjs";
import { DEFAULT_MARKUP_BPS, MAX_MARKUP_BPS } from "./money.mjs";
import { prepare } from "./prepare.mjs";
import { createLimiters } from "./ratelimit.mjs";
import { send } from "./send.mjs";

const BODY_LIMIT_BYTES = 16 * 1024;

const requireObject = (body) => {
  if (!body || typeof body !== "object" || Array.isArray(body)) throw new Refusal("bad_request", "the body must be a JSON object");
  return body;
};

const requireText = (value, what) => {
  if (typeof value !== "string" || value.length === 0) throw new Refusal("bad_request", `${what} is required`);
  return value;
};

export function publicConfig(config) {
  return {
    network: config.network,
    feePayer: config.feePayer.toBase58(),
    usdcMint: config.usdcMint.toBase58(),
    charge: config.charge,
    markup: { defaultBps: DEFAULT_MARKUP_BPS, maxBps: MAX_MARKUP_BPS },
    split: { appBps: 8000, noirwireBps: 2000 },
  };
}

/**
 * `ctx`: config, chain, store, kora, quotes, challenges, sessions, readPrice, now, and
 * optionally limiters and a logger.
 */
export async function buildServer(ctx) {
  const { config } = ctx;
  ctx.limiters ??= createLimiters(config, ctx.now);
  ctx.sessions ??= new Sessions({ now: ctx.now });
  const { limiters } = ctx;
  const perIp = (request) => {
    if (!limiters.ip.allow(request.ip)) throw new Refusal("rate_limited", "too many requests, slow down");
  };
  const perIpForAccounts = (request) => {
    if (!limiters.registration.allow(request.ip)) throw new Refusal("rate_limited", "too many requests, slow down");
  };

  // Trust exactly the configured number of hops counted from the socket (hop 0 is the socket
  // itself). The client address is then the right-most X-Forwarded-For entry that a trusted hop
  // did not vouch for; anything a client put to the left of it is ignored. A plain number is not
  // passed to Fastify because current Fastify refuses to trust a bare hop count.
  const trusted = (address, hop) => hop < config.trustedProxyHops;
  const app = Fastify({ logger: ctx.logger ?? { level: "warn" }, trustProxy: config.trustedProxyHops > 0 ? trusted : false, bodyLimit: BODY_LIMIT_BYTES });
  await app.register(cors, { origin: true, methods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"] });

  app.addHook("onRequest", async () => {
    if (!limiters.global.allow("all")) throw new Refusal("overloaded", "the service is busy, try again shortly");
  });

  app.setErrorHandler((error, request, reply) => {
    if (error instanceof Refusal) return reply.code(error.status).send({ error: error.message, code: error.code });
    if (error.statusCode && error.statusCode < 500) return reply.code(error.statusCode).send({ error: error.message, code: "bad_request" });
    request.log.error({ message: error.message }, "unhandled error");
    return reply.code(500).send({ error: "internal error", code: "internal" });
  });
  app.setNotFoundHandler((request, reply) => reply.code(404).send({ error: "not found", code: "not_found" }));

  app.get("/v1/health", async () => {
    await ctx.store.ping();
    return { ok: true, network: config.network, feePayer: healthView(ctx) };
  });

  app.get("/v1/config", async () => publicConfig(config));

  app.post("/v1/prepare", async (request) => {
    const body = requireObject(request.body);
    perIp(request);
    if (!limiters.user.allow(`${body.user}`)) throw new Refusal("rate_limited", "too many requests, slow down");
    return prepare(ctx, {
      transaction: requireText(body.transaction, "transaction"),
      user: body.user,
      appId: body.app,
      origin: request.headers.origin,
      clientIp: request.ip,
    });
  });

  app.post("/v1/send", async (request, reply) => {
    const body = requireObject(request.body);
    perIp(request);
    const transaction = requireText(body.transaction, "transaction");
    const tx = decodeTransaction(transaction, config.caps.maxTxBytes);
    const result = await send(ctx, { tx, transaction, clientIp: request.ip, origin: request.headers.origin });
    return reply.code(result.status).send(result.body);
  });

  app.post("/v1/auth/challenge", async (request) => {
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return issueSignInChallenge(ctx, body);
  });

  app.post("/v1/auth/session", async (request) => {
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return openSession(ctx, body);
  });

  app.delete("/v1/auth/session", async (request, reply) => {
    authenticate(ctx, request);
    ctx.sessions.revoke(bearerToken(request));
    return reply.code(204).send();
  });

  app.get("/v1/me/apps", async (request) => {
    const owner = authenticate(ctx, request);
    perIp(request);
    return listApps(ctx, owner);
  });

  app.post("/v1/apps", async (request, reply) => {
    const owner = authenticate(ctx, request);
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return reply.code(201).send(await createApp(ctx, owner, body));
  });

  app.get("/v1/apps/:id", async (request) => {
    const owner = authenticate(ctx, request);
    perIp(request);
    return getApp(ctx, owner, request.params.id);
  });

  app.patch("/v1/apps/:id", async (request) => {
    const owner = authenticate(ctx, request);
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return patchApp(ctx, owner, request.params.id, body);
  });

  app.post("/v1/apps/:id/payout/challenge", async (request) => {
    const owner = authenticate(ctx, request);
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return issuePayoutChallenge(ctx, owner, request.params.id, body);
  });

  app.put("/v1/apps/:id/payout", async (request) => {
    const owner = authenticate(ctx, request);
    const body = requireObject(request.body);
    perIpForAccounts(request);
    return setPayout(ctx, owner, request.params.id, body);
  });

  app.get("/v1/apps/:id/stats", async (request) => {
    const owner = authenticate(ctx, request);
    perIp(request);
    return appStats(ctx, owner, request.params.id, request.query.window);
  });

  app.get("/v1/apps/:id/activity", async (request) => {
    const owner = authenticate(ctx, request);
    perIp(request);
    return appActivity(ctx, owner, request.params.id, request.query);
  });

  return app;
}
