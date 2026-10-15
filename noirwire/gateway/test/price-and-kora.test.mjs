import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import { test } from "node:test";
import { Refusal, UnknownOutcome } from "../src/errors.mjs";
import { createKoraClient } from "../src/kora.mjs";
import { readReferencePrice } from "../src/price.mjs";
import { accountInfo, PYTH_RECEIVER, pythAccount, wallet } from "./helpers.mjs";

const NOW = 1_800_000_000;
const chainWith = (info) => ({ getAccountInfo: async () => info });

test("a fresh, verified Pyth price reads as micro-USDC per SOL", async () => {
  assert.equal(await readReferencePrice(chainWith(pythAccount({ publishTime: NOW - 5 })), NOW), 150_000_000n);
});

test("a stale price fails closed", async () => {
  await assert.rejects(readReferencePrice(chainWith(pythAccount({ publishTime: NOW - 600 })), NOW), { code: "no_price" });
});

test("an unverified, wrong-owner, missing or wide-interval price is refused", async () => {
  const fresh = NOW - 1;
  await assert.rejects(readReferencePrice(chainWith(pythAccount({ publishTime: fresh, verification: 0 })), NOW), { code: "no_price" });
  await assert.rejects(readReferencePrice(chainWith(pythAccount({ publishTime: fresh, owner: wallet().publicKey })), NOW), { code: "no_price" });
  await assert.rejects(readReferencePrice(chainWith(null), NOW), { code: "no_price" });
  await assert.rejects(readReferencePrice(chainWith(pythAccount({ publishTime: fresh, confidence: 500_000_000n })), NOW), { code: "no_price" });
  await assert.rejects(readReferencePrice(chainWith(accountInfo(PYTH_RECEIVER, Buffer.alloc(10))), NOW), { code: "no_price" });
});

function fakeFetch(responder) {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, init });
    return responder(init);
  };
  return { fetchImpl, calls };
}

const reply = (status, body) => ({ status, ok: status >= 200 && status < 300, text: async () => (typeof body === "string" ? body : JSON.stringify(body)) });

test("kora: the request carries the API key and an HMAC over timestamp + body", async () => {
  const { fetchImpl, calls } = fakeFetch(() => reply(200, { jsonrpc: "2.0", id: 1, result: { signed_transaction: "SIGNED", signer_pubkey: "x" } }));
  const kora = createKoraClient({ url: "http://kora", apiKey: "the-key", hmacSecret: "the-secret", fetchImpl, now: () => 1_800_000_123_456 });
  assert.equal(await kora.signTransaction("TXB64"), "SIGNED");
  const { url, init } = calls[0];
  assert.equal(url, "http://kora");
  assert.equal(init.method, "POST");
  assert.deepEqual(JSON.parse(init.body), { id: 1, jsonrpc: "2.0", method: "signTransaction", params: { transaction: "TXB64" } });
  assert.equal(init.headers["x-api-key"], "the-key");
  assert.equal(init.headers["x-timestamp"], "1800000123");
  const expected = createHmac("sha256", "the-secret").update("1800000123" + init.body).digest("hex");
  assert.equal(init.headers["x-hmac-signature"], expected);
});

test("kora: an error answer is a refusal, nothing was signed", async () => {
  const jsonRpcError = fakeFetch(() => reply(200, { jsonrpc: "2.0", id: 1, error: { code: -32000, message: "policy says no" } }));
  const a = createKoraClient({ url: "http://kora", apiKey: "k", hmacSecret: "s", fetchImpl: jsonRpcError.fetchImpl });
  await assert.rejects(a.signTransaction("x"), (error) => error instanceof Refusal && error.code === "kora_rejected" && /policy says no/.test(error.message));

  const unauthorized = fakeFetch(() => reply(401, ""));
  const b = createKoraClient({ url: "http://kora", apiKey: "k", hmacSecret: "s", fetchImpl: unauthorized.fetchImpl });
  await assert.rejects(b.signTransaction("x"), { code: "kora_rejected" });
});

test("kora: a timeout, dropped connection, server error or unreadable answer is an unknown outcome", async () => {
  const cases = [
    () => {
      throw new Error("socket hang up");
    },
    () => reply(502, "bad gateway"),
    () => reply(200, "<html>"),
    () => reply(200, { jsonrpc: "2.0", id: 1, result: {} }),
    () => reply(500, { jsonrpc: "2.0", id: 1 }),
  ];
  for (const responder of cases) {
    const { fetchImpl } = fakeFetch(responder);
    const kora = createKoraClient({ url: "http://kora", apiKey: "k", hmacSecret: "s", fetchImpl });
    await assert.rejects(kora.signTransaction("x"), (error) => error instanceof UnknownOutcome);
  }
});
