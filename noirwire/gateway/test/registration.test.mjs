import assert from "node:assert/strict";
import { test } from "node:test";
import { ed25519 } from "@noble/curves/ed25519";
import { Keypair, PublicKey } from "@solana/web3.js";
import { parseDailyCap, parseMarkupBps, parseName, parseOrigins, parsePrograms, parseWallet, payoutMessage, signInMessage, walletSignatureValid } from "../src/registration.mjs";
import { base58Encode } from "../src/units.mjs";

const sign = (keypair, text) => base58Encode(ed25519.sign(new TextEncoder().encode(text), keypair.secretKey.slice(0, 32)));

test("the sign-in message names the wallet, network, nonce and time, and says it is not a transaction", () => {
  const text = signInMessage({ wallet: "WALLET", network: "devnet", nonce: "n1", issuedAt: "2026-10-15T10:00:00.000Z" });
  assert.match(text, /^noirwire relayer wants you to sign in with your Solana wallet:\nWALLET\n/);
  assert.match(text, /not a transaction/);
  assert.match(text, /Network: devnet\nNonce: n1\nIssued At: 2026-10-15T10:00:00.000Z$/);
});

test("the payout message names the app, the owner and the new wallet", () => {
  const text = payoutMessage({ appId: "app_x", owner: "OWNER", payoutWallet: "NEW", network: "mainnet", nonce: "n", issuedAt: "t" });
  assert.match(text, /App: app_x\nOwner wallet: OWNER\nNew payout wallet: NEW/);
  assert.match(text, /not a transaction/);
});

test("a wallet signature verifies over the exact text, and any change breaks it", () => {
  const wallet = Keypair.generate();
  const text = signInMessage({ wallet: wallet.publicKey.toBase58(), network: "devnet", nonce: "n1", issuedAt: "t" });
  const signatureBase58 = sign(wallet, text);
  const check = (overrides) => walletSignatureValid({ wallet: wallet.publicKey.toBase58(), text, signatureBase58, ...overrides });
  assert.equal(check({}), true);
  assert.equal(check({ text: text.replace("n1", "n2") }), false);
  assert.equal(check({ wallet: Keypair.generate().publicKey.toBase58() }), false);
  assert.equal(check({ signatureBase58: sign(Keypair.generate(), text) }), false);
  assert.equal(check({ signatureBase58: "not base58 !!" }), false);
  assert.equal(check({ signatureBase58: undefined }), false);
  assert.equal(check({ signatureBase58: base58Encode(new Uint8Array(64)) }), false);
});

test("markup must be a whole number from 0 to 30000", () => {
  assert.equal(parseMarkupBps(0), 0);
  assert.equal(parseMarkupBps(30_000), 30_000);
  for (const bad of [-1, 30_001, 1.5, "100", null, undefined, NaN]) assert.throws(() => parseMarkupBps(bad), { code: "bad_request" });
});

test("names cannot carry line breaks that would forge lines of a signed message", () => {
  assert.equal(parseName("  Acme  "), "Acme");
  for (const bad of ["", "   ", "a\nMarkup: 0", "a\u0000b", "x".repeat(65), 5, null]) assert.throws(() => parseName(bad), { code: "bad_request" });
});

test("signers must be wallets, payout addresses may be program-owned", () => {
  const [derived] = PublicKey.findProgramAddressSync([Buffer.from("treasury")], Keypair.generate().publicKey);
  assert.ok(parseWallet(Keypair.generate().publicKey.toBase58(), "owner", { mustSign: true }));
  assert.throws(() => parseWallet("nonsense", "owner", { mustSign: true }), { code: "bad_request" });
  assert.throws(() => parseWallet(42, "payoutWallet"), { code: "bad_request" });
  assert.throws(() => parseWallet(derived.toBase58(), "owner", { mustSign: true }), { code: "bad_request" });
  assert.ok(parseWallet(derived.toBase58(), "payoutWallet"));
});

test("allowed origins are bare origins", () => {
  assert.deepEqual(parseOrigins(["https://app.example.com", "http://localhost:3000"]), ["https://app.example.com", "http://localhost:3000"]);
  for (const bad of ["app.example.com", "https://app.example.com/path", "ftp://x.example", "https://app.example.com/", "*", 5]) {
    assert.throws(() => parseOrigins([bad]), { code: "bad_request" }, String(bad));
  }
  assert.throws(() => parseOrigins("https://x.example"), { code: "bad_request" });
  assert.throws(() => parseOrigins(Array.from({ length: 21 }, (_, i) => `https://a${i}.example`)), { code: "bad_request" });
});

test("allowed programs are public keys", () => {
  const program = Keypair.generate().publicKey.toBase58();
  assert.deepEqual(parsePrograms([program]), [program]);
  assert.throws(() => parsePrograms(["nope"]), { code: "bad_request" });
  assert.throws(() => parsePrograms("x"), { code: "bad_request" });
});

test("the daily cap is a whole number of lamports within the service maximum", () => {
  assert.equal(parseDailyCap("5000", 10_000n), 5000n);
  assert.equal(parseDailyCap(5000, 10_000n), 5000n);
  for (const bad of ["0", "10001", "-1", "1.5", "abc", null, undefined]) assert.throws(() => parseDailyCap(bad, 10_000n), { code: "bad_request" }, String(bad));
});
