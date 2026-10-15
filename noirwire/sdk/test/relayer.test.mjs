import assert from "node:assert/strict";
import { test } from "node:test";
import { Keypair, SystemProgram, Transaction, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { createRelayer, NETWORK_URLS, RelayerError, SendOutcomeUnknownError } from "../dist/index.js";

const BLOCKHASH = Keypair.generate().publicKey.toBase58();
const feePayer = Keypair.generate();
const user = Keypair.generate();

const legacy = () => {
  const tx = new Transaction({ feePayer: feePayer.publicKey, recentBlockhash: BLOCKHASH });
  tx.add(SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: Keypair.generate().publicKey, lamports: 1 }));
  return tx;
};

const versioned = () =>
  new VersionedTransaction(
    new TransactionMessage({
      payerKey: feePayer.publicKey,
      recentBlockhash: BLOCKHASH,
      instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: Keypair.generate().publicKey, lamports: 1 })],
    }).compileToV0Message(),
  );

const reply = (status, body) => ({ status, json: async () => body });

function fakeFetch(handler) {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, method: init?.method ?? "GET", body: init?.body ? JSON.parse(init.body) : undefined });
    return handler(url, init, calls.length);
  };
  return { fetchImpl, calls };
}

const QUOTE = { networkCostMicroUsdc: "1500", markupMicroUsdc: "1500", appMicroUsdc: "1200", noirwireMicroUsdc: "300", totalMicroUsdc: "3000" };

const config = {
  network: "devnet",
  feePayer: feePayer.publicKey.toBase58(),
  usdcMint: Keypair.generate().publicKey.toBase58(),
  charge: false,
  markup: { defaultBps: 10000, maxBps: 30000 },
  split: { appBps: 8000, noirwireBps: 2000 },
};

test("the network picks the deployment, a url overrides it, and trailing slashes are ignored", async () => {
  const { fetchImpl, calls } = fakeFetch(() => reply(200, config));
  await createRelayer({ app: "app_x", network: "mainnet", fetch: fetchImpl }).config();
  await createRelayer({ app: "app_x", fetch: fetchImpl }).config();
  await createRelayer({ app: "app_x", url: "https://relayer.example/", fetch: fetchImpl }).config();
  assert.deepEqual(
    calls.map((c) => c.url),
    [`${NETWORK_URLS.mainnet}/v1/config`, `${NETWORK_URLS.devnet}/v1/config`, "https://relayer.example/v1/config"],
  );
});

test("feePayer reads the config once and returns a PublicKey", async () => {
  const { fetchImpl, calls } = fakeFetch(() => reply(200, config));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  assert.equal((await relayer.feePayer()).toBase58(), feePayer.publicKey.toBase58());
  await relayer.feePayer();
  assert.equal(calls.length, 1);
});

test("a failed config read is not cached", async () => {
  const { fetchImpl, calls } = fakeFetch((url, init, n) => (n === 1 ? reply(503, { error: "busy", code: "overloaded" }) : reply(200, config)));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  await assert.rejects(relayer.feePayer(), (error) => error instanceof RelayerError && error.code === "overloaded" && error.status === 503);
  await relayer.feePayer();
  assert.equal(calls.length, 2);
});

for (const [name, make] of [["legacy", legacy], ["versioned", versioned]]) {
  test(`prepare sends the app, the unsigned ${name} transaction and the user, and returns the same kind of transaction`, async () => {
    const original = make();
    const returned = make();
    const { fetchImpl, calls } = fakeFetch(() =>
      reply(200, {
        transaction: Buffer.from(returned instanceof Transaction ? returned.serialize({ requireAllSignatures: false, verifySignatures: false }) : returned.serialize()).toString("base64"),
        quote: QUOTE,
        expiresAt: "2026-10-15T10:00:00.000Z",
      }),
    );
    const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
    const prepared = await relayer.prepare(original, user.publicKey);
    assert.equal(calls[0].url, "https://r.example/v1/prepare");
    assert.equal(calls[0].method, "POST");
    assert.deepEqual(Object.keys(calls[0].body).sort(), ["app", "transaction", "user"]);
    assert.equal(calls[0].body.app, "app_x");
    assert.equal(calls[0].body.user, user.publicKey.toBase58());
    assert.equal(prepared.transaction instanceof Transaction, original instanceof Transaction);
    assert.deepEqual(prepared.quote, QUOTE);
    assert.equal(prepared.expiresAt, "2026-10-15T10:00:00.000Z");
  });
}

test("prepare errors carry the relayer's code and status, and are not retried", async () => {
  const { fetchImpl, calls } = fakeFetch(() => reply(403, { error: "this app is paused", code: "app_paused" }));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  await assert.rejects(relayer.prepare(legacy(), user.publicKey), (error) => error instanceof RelayerError && error.code === "app_paused" && error.status === 403 && /paused/.test(error.message));
  assert.equal(calls.length, 1);
});

function walletThatSigns() {
  const signed = [];
  return {
    signed,
    publicKey: user.publicKey,
    async signTransaction(transaction) {
      signed.push(transaction);
      transaction instanceof Transaction ? transaction.partialSign(user) : transaction.sign([user]);
      return transaction;
    },
  };
}

function prepareThenSend(sendReply) {
  const prepared = legacy();
  const wire = Buffer.from(prepared.serialize({ requireAllSignatures: false, verifySignatures: false })).toString("base64");
  return fakeFetch((url) => (url.endsWith("/v1/prepare") ? reply(200, { transaction: wire, quote: QUOTE, expiresAt: "t" }) : sendReply(url)));
}

test("signAndSend prepares, has the wallet sign exactly the prepared transaction, then sends it once", async () => {
  const { fetchImpl, calls } = prepareThenSend(() => reply(200, { signature: "SIG", status: "sent" }));
  const wallet = walletThatSigns();
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  const result = await relayer.signAndSend(legacy(), wallet);
  assert.deepEqual(result, { outcome: "sent", signature: "SIG", status: "sent" });
  assert.deepEqual(calls.map((c) => c.url), ["https://r.example/v1/prepare", "https://r.example/v1/send"]);
  assert.equal(wallet.signed.length, 1);
  const sentBytes = Buffer.from(calls[1].body.transaction, "base64");
  const received = Transaction.from(sentBytes);
  assert.ok(received.signatures.find((s) => s.publicKey.equals(user.publicKey))?.signature, "the user's signature is in what was sent");
  assert.deepEqual(Object.keys(calls[1].body), ["transaction"]);
});

test("an unknown outcome (202) is returned as a typed result and nothing is sent again", async () => {
  const { fetchImpl, calls } = prepareThenSend(() => reply(202, { outcome: "unknown", messageHash: "abc", signature: "SIG" }));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  const result = await relayer.signAndSend(legacy(), walletThatSigns());
  assert.deepEqual(result, { outcome: "unknown", messageHash: "abc", signature: "SIG" });
  assert.equal(calls.filter((c) => c.url.endsWith("/v1/send")).length, 1);

  const bare = prepareThenSend(() => reply(202, { outcome: "unknown", messageHash: "abc" }));
  const second = await createRelayer({ app: "app_x", url: "https://r.example", fetch: bare.fetchImpl }).signAndSend(legacy(), walletThatSigns());
  assert.deepEqual(second, { outcome: "unknown", messageHash: "abc" });
});

test("a dropped connection on send is reported as unknown and is never retried", async () => {
  const { fetchImpl, calls } = prepareThenSend(() => {
    throw new Error("socket hang up");
  });
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  await assert.rejects(relayer.signAndSend(legacy(), walletThatSigns()), SendOutcomeUnknownError);
  assert.equal(calls.filter((c) => c.url.endsWith("/v1/send")).length, 1);
});

test("a send the relayer refuses throws its error once", async () => {
  const { fetchImpl, calls } = prepareThenSend(() => reply(404, { error: "no live quote", code: "no_quote" }));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  await assert.rejects(relayer.signAndSend(legacy(), walletThatSigns()), (error) => error instanceof RelayerError && error.code === "no_quote");
  assert.equal(calls.filter((c) => c.url.endsWith("/v1/send")).length, 1);
});

test("after a send is dispatched, a 5xx, a non-JSON body or an unexpected answer is an unknown outcome, never a RelayerError", async () => {
  const unclear = [
    ["a 502 from a proxy after the broadcast", () => reply(502, { error: "bad gateway", code: "upstream" })],
    ["a 500", () => reply(500, { error: "internal error", code: "internal" })],
    ["a 503 with a code the relayer does not give before signing", () => reply(503, { error: "x", code: "unavailable" })],
    ["a 502 with an HTML body", () => ({ status: 502, json: async () => { throw new Error("not json"); } })],
    ["a 4xx without a code", () => reply(429, {})],
    ["a 200 that carries no signature", () => reply(200, { ok: true })],
    ["a 202 without a message hash", () => reply(202, {})],
  ];
  for (const [name, answer] of unclear) {
    const { fetchImpl, calls } = prepareThenSend(answer);
    const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
    await assert.rejects(relayer.signAndSend(legacy(), walletThatSigns()), (error) => error instanceof SendOutcomeUnknownError && !(error instanceof RelayerError), name);
    assert.equal(calls.filter((c) => c.url.endsWith("/v1/send")).length, 1, `${name}: sent once`);
  }
});

test("a documented refusal before signing is a RelayerError: 4xx with a code, and the 503s given before the signer is called", async () => {
  for (const [status, code] of [[403, "app_paused"], [404, "no_quote"], [409, "app_settings_changed"], [422, "kora_rejected"], [429, "wallet_loss_budget"], [503, "loss_ceiling"], [503, "reconciliation_unhealthy"], [503, "overloaded"]]) {
    const { fetchImpl } = prepareThenSend(() => reply(status, { error: "refused", code }));
    const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
    await assert.rejects(relayer.signAndSend(legacy(), walletThatSigns()), (error) => error instanceof RelayerError && error.code === code && error.status === status, `${status} ${code}`);
  }
});

test("an error body that is not JSON still becomes a RelayerError", async () => {
  const { fetchImpl } = fakeFetch(() => ({ status: 502, json: async () => { throw new Error("not json"); } }));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  await assert.rejects(relayer.prepare(legacy(), user.publicKey), (error) => error instanceof RelayerError && error.status === 502 && error.code === "unknown_error");
});

test("send takes a transaction the caller signed themselves", async () => {
  const { fetchImpl, calls } = fakeFetch(() => reply(200, { signature: "SIG", status: "sent" }));
  const relayer = createRelayer({ app: "app_x", url: "https://r.example", fetch: fetchImpl });
  const tx = versioned();
  tx.sign([user]);
  assert.equal((await relayer.send(tx)).outcome, "sent");
  assert.equal(calls[0].url, "https://r.example/v1/send");
});
