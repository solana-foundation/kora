import { isReady } from "./access.mjs";
import { Refusal } from "./errors.mjs";
import { DEFAULT_MARKUP_BPS } from "./money.mjs";
import { usdcAccountOf } from "./payments.mjs";
import {
  newAppId,
  parseDailyCap,
  parseMarkupBps,
  parseName,
  parseOrigins,
  parsePrograms,
  parseWallet,
  payoutMessage,
  walletSignatureValid,
} from "./registration.mjs";
import { DAY_MS, startOfUtcDay } from "./units.mjs";

const MAX_APPS_PER_OWNER = 20;
const DEFAULT_ACTIVITY_LIMIT = 50;
const MAX_ACTIVITY_LIMIT = 100;
const WINDOW_DAYS = { today: 1, "30d": 30 };
const PENDING = new Set(["sending", "signed", "sent"]);

export function appView(app, config) {
  return {
    id: app.id,
    name: app.name,
    owner: app.owner,
    network: config.network,
    payoutWallet: app.payoutWallet,
    markupBps: app.markupBps,
    allowedOrigins: app.allowedOrigins,
    allowedPrograms: app.allowedPrograms,
    dailyCapLamports: app.dailyCapLamports.toString(),
    paused: app.paused,
    ready: isReady(app, config),
    createdAt: new Date(app.createdAt).toISOString(),
    firstTransactionAt: app.firstTransactionAt === null ? null : new Date(app.firstTransactionAt).toISOString(),
  };
}

/** An app the signed-in wallet owns. Anyone else's is reported as missing. */
async function ownedApp(ctx, owner, appId) {
  const app = typeof appId === "string" ? await ctx.store.getApp(appId) : null;
  if (!app || app.owner !== owner) throw new Refusal("not_found", "no such app");
  return app;
}

export async function listApps(ctx, owner) {
  return { apps: (await ctx.store.listAppsByOwner(owner)).map((app) => appView(app, ctx.config)) };
}

export async function createApp(ctx, owner, body) {
  const name = parseName(body.name);
  if ((await ctx.store.listAppsByOwner(owner)).length >= MAX_APPS_PER_OWNER) {
    throw new Refusal("app_limit_reached", `a wallet can own at most ${MAX_APPS_PER_OWNER} apps`);
  }
  const app = {
    id: newAppId(),
    owner,
    name,
    payoutWallet: null,
    markupBps: DEFAULT_MARKUP_BPS,
    allowedOrigins: [],
    allowedPrograms: [],
    dailyCapLamports: ctx.config.apps.dailyCapDefaultLamports,
    paused: false,
  };
  await ctx.store.createApp(app);
  return appView(await ctx.store.getApp(app.id), ctx.config);
}

export async function getApp(ctx, owner, appId) {
  return appView(await ownedApp(ctx, owner, appId), ctx.config);
}

export async function patchApp(ctx, owner, appId, body) {
  await ownedApp(ctx, owner, appId);
  const fields = {};
  if (body.name !== undefined) fields.name = parseName(body.name);
  if (body.markupBps !== undefined) fields.markupBps = parseMarkupBps(body.markupBps);
  if (body.allowedOrigins !== undefined) fields.allowedOrigins = parseOrigins(body.allowedOrigins);
  if (body.allowedPrograms !== undefined) fields.allowedPrograms = parsePrograms(body.allowedPrograms);
  if (body.dailyCapLamports !== undefined) fields.dailyCapLamports = parseDailyCap(body.dailyCapLamports, ctx.config.apps.dailyCapMaxLamports);
  if (body.paused !== undefined) {
    if (typeof body.paused !== "boolean") throw new Refusal("bad_request", "paused must be true or false");
    fields.paused = body.paused;
  }
  await ctx.store.updateApp(appId, fields);
  return getApp(ctx, owner, appId);
}

async function requirePayoutAccount(ctx, payoutWallet) {
  const account = await ctx.chain.getAccountInfo(usdcAccountOf(ctx.config.usdcMint, payoutWallet));
  if (!account) throw new Refusal("no_payout_account", "the payout wallet has no USDC account yet; create it first");
}

/** The text the owner must sign to point this app's earnings at a new wallet. */
export async function issuePayoutChallenge(ctx, owner, appId, body) {
  await ownedApp(ctx, owner, appId);
  const payoutWallet = parseWallet(body.payoutWallet, "payoutWallet").toBase58();
  const { network } = ctx.config;
  const { nonce, message, expiresAt } = ctx.challenges.issue({
    meta: { kind: "payout", wallet: owner, appId, payoutWallet },
    buildMessage: (nonce, issuedAt) => payoutMessage({ appId, owner, payoutWallet, network, nonce, issuedAt }),
  });
  return { nonce, message, expiresAt: new Date(expiresAt).toISOString() };
}

export async function setPayout(ctx, owner, appId, body) {
  await ownedApp(ctx, owner, appId);
  const payoutWallet = parseWallet(body.payoutWallet, "payoutWallet");
  const challenge = typeof body.nonce === "string" ? ctx.challenges.peek(body.nonce) : null;
  const meta = challenge?.meta;
  if (!meta || meta.kind !== "payout" || meta.wallet !== owner || meta.appId !== appId || meta.payoutWallet !== payoutWallet.toBase58()) {
    throw new Refusal("bad_signature", "the challenge is unknown, already used, expired or for something else");
  }
  if (!walletSignatureValid({ wallet: owner, text: challenge.message, signatureBase58: body.signature })) {
    throw new Refusal("bad_signature", "the signature does not match the payout message");
  }
  if (!ctx.challenges.consume(body.nonce)) throw new Refusal("bad_signature", "the challenge is unknown, already used or expired");
  await requirePayoutAccount(ctx, payoutWallet);
  await ctx.store.updateApp(appId, { payoutWallet: payoutWallet.toBase58() });
  return getApp(ctx, owner, appId);
}

const dayKey = (ms) => new Date(ms).toISOString().slice(0, 10);

export async function appStats(ctx, owner, appId, windowParam = "today") {
  await ownedApp(ctx, owner, appId);
  const days = WINDOW_DAYS[windowParam];
  if (!days) throw new Refusal("bad_request", "window must be today or 30d");
  const today = startOfUtcDay(ctx.now());
  const since = today - (days - 1) * DAY_MS;
  const rows = new Map((await ctx.store.appDaily(appId, since)).map((row) => [row.day, row]));
  const totals = { transactions: 0, failed: 0, volume: 0n, earned: 0n };
  const daily = [];
  for (let i = 0; i < days; i += 1) {
    const day = dayKey(since + i * DAY_MS);
    const row = rows.get(day) ?? { landed: 0, failed: 0, volume: 0n, earned: 0n };
    totals.transactions += row.landed;
    totals.failed += row.failed;
    totals.volume += row.volume;
    totals.earned += row.earned;
    daily.push({ day, transactions: row.landed, failed: row.failed, earnedMicroUsdc: row.earned.toString() });
  }
  return {
    window: windowParam,
    transactions: totals.transactions,
    failed: totals.failed,
    volumeMicroUsdc: totals.volume.toString(),
    earnedMicroUsdc: totals.earned.toString(),
    daily,
  };
}

const encodeCursor = (row) => Buffer.from(JSON.stringify({ c: row.createdAt, h: row.messageHash })).toString("base64url");

function decodeCursor(text) {
  if (text === undefined) return null;
  try {
    const { c, h } = JSON.parse(Buffer.from(text, "base64url").toString("utf8"));
    if (Number.isFinite(c) && typeof h === "string") return { createdAt: c, messageHash: h };
  } catch {
    // handled below
  }
  throw new Refusal("bad_request", "before is not a cursor this service issued");
}

const activityStatus = (status) => (PENDING.has(status) ? "pending" : status);

export async function appActivity(ctx, owner, appId, query) {
  await ownedApp(ctx, owner, appId);
  const limit = query.limit === undefined ? DEFAULT_ACTIVITY_LIMIT : Number(query.limit);
  if (!Number.isInteger(limit) || limit < 1 || limit > MAX_ACTIVITY_LIMIT) {
    throw new Refusal("bad_request", `limit must be a whole number from 1 to ${MAX_ACTIVITY_LIMIT}`);
  }
  const before = decodeCursor(query.before);
  const rows = await ctx.store.activity(appId, before, limit + 1);
  const page = rows.slice(0, limit);
  return {
    items: page.map((row) => ({
      createdAt: new Date(row.createdAt).toISOString(),
      status: activityStatus(row.status),
      signature: row.signature,
      totalMicroUsdc: (row.networkCost + row.markup).toString(),
      appMicroUsdc: row.app.toString(),
      networkCostMicroUsdc: row.networkCost.toString(),
      failureReason: row.failureReason,
    })),
    next: rows.length > limit ? encodeCursor(page.at(-1)) : null,
  };
}
