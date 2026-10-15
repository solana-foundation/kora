import assert from "node:assert/strict";
import { test } from "node:test";
import { SystemProgram, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { appendInstructions, decodeTransaction, describeMessage, messageHash, userSignaturesValid } from "../src/message.mjs";
import { buildPaymentInstructions, usdcAccountOf } from "../src/payments.mjs";
import { buildMessage, buildTransaction, base64Of, lookupTable, programCall, wallet } from "./helpers.mjs";

test("the message hash changes with any change to the message", () => {
  const payer = wallet();
  const user = wallet();
  const to = wallet().publicKey;
  const a = buildMessage({ payer: payer.publicKey, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: to, lamports: 1 })] });
  const same = buildMessage({ payer: payer.publicKey, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: to, lamports: 1 })] });
  const other = buildMessage({ payer: payer.publicKey, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: to, lamports: 2 })] });
  assert.equal(messageHash(a), messageHash(same));
  assert.notEqual(messageHash(a), messageHash(other));
  assert.match(messageHash(a), /^[0-9a-f]{64}$/);
});

test("the hash ignores signatures, which are not part of the message", () => {
  const payer = wallet();
  const user = wallet();
  const tx = buildTransaction({ payer: payer.publicKey, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: wallet().publicKey, lamports: 1 })] });
  const before = messageHash(tx.message);
  tx.sign([user]);
  assert.equal(messageHash(tx.message), before);
});

test("legacy and v0 messages both hash", () => {
  const payer = wallet().publicKey;
  const ix = [SystemProgram.transfer({ fromPubkey: payer, toPubkey: wallet().publicKey, lamports: 1 })];
  assert.notEqual(messageHash(buildMessage({ payer, instructions: ix, legacy: true })), messageHash(buildMessage({ payer, instructions: ix })));
});

test("user signatures: valid, missing, wrong, and the fee payer's slot may be empty", () => {
  const payer = wallet();
  const user = wallet();
  const ix = [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: wallet().publicKey, lamports: 1 })];
  const tx = buildTransaction({ payer: payer.publicKey, instructions: ix });
  assert.equal(userSignaturesValid(tx), false, "unsigned");
  tx.sign([user]);
  assert.equal(userSignaturesValid(tx), true, "signed by the user, fee payer slot empty");

  const forged = buildTransaction({ payer: payer.publicKey, instructions: ix });
  forged.sign([wallet(), user].slice(1));
  forged.signatures[1] = Uint8Array.from(forged.signatures[1]).map((byte, i) => (i === 0 ? byte ^ 1 : byte));
  assert.equal(userSignaturesValid(forged), false, "tampered signature");

  const otherMessage = buildTransaction({ payer: payer.publicKey, instructions: [SystemProgram.transfer({ fromPubkey: user.publicKey, toPubkey: wallet().publicKey, lamports: 2 })] });
  otherMessage.sign([user]);
  const moved = buildTransaction({ payer: payer.publicKey, instructions: ix });
  moved.signatures[1] = otherMessage.signatures[1];
  assert.equal(userSignaturesValid(moved), false, "a signature over another message");
});

test("every non-fee-payer required signer must have signed", () => {
  const payer = wallet();
  const user = wallet();
  const extra = wallet();
  const ix = [programCall(wallet().publicKey, [{ pubkey: user.publicKey, isSigner: true }, { pubkey: extra.publicKey, isSigner: true }])];
  const tx = buildTransaction({ payer: payer.publicKey, instructions: ix });
  tx.sign([user]);
  assert.equal(userSignaturesValid(tx), false);
  tx.sign([extra]);
  assert.equal(userSignaturesValid(tx), true);
});

test("a malformed transaction is refused, and so is an oversized one", () => {
  assert.throws(() => decodeTransaction("not base64!", 1232), { code: "bad_transaction" });
  assert.throws(() => decodeTransaction("AAAA", 1232), { code: "bad_transaction" });
  assert.throws(() => decodeTransaction("", 1232), { code: "bad_transaction" });
  const payer = wallet().publicKey;
  const tx = buildTransaction({ payer, instructions: [SystemProgram.transfer({ fromPubkey: payer, toPubkey: wallet().publicKey, lamports: 1 })] });
  assert.throws(() => decodeTransaction(base64Of(tx), 50), { code: "too_large" });
  assert.ok(decodeTransaction(base64Of(tx), 1232) instanceof VersionedTransaction);
});

function paymentsFor(user, mint) {
  return buildPaymentInstructions({
    user,
    mint,
    noirwireAccount: wallet().publicKey,
    appAccount: wallet().publicKey,
    split: { networkCost: 700n, markup: 700n, noirwire: 140n, app: 560n, total: 1400n },
  });
}

test("payments: B + S to NoirWire and C to the app, from the user's USDC account", () => {
  const user = wallet().publicKey;
  const mint = wallet().publicKey;
  const instructions = paymentsFor(user, mint);
  assert.equal(instructions.length, 2);
  const [toNoirwire, toApp] = instructions;
  assert.equal(toNoirwire.data.readBigUInt64LE(1), 840n);
  assert.equal(toApp.data.readBigUInt64LE(1), 560n);
  for (const ix of instructions) {
    assert.ok(ix.keys[0].pubkey.equals(usdcAccountOf(mint, user)));
    assert.ok(ix.keys[3].pubkey.equals(user));
    assert.equal(ix.keys[3].isSigner, true);
    assert.equal(ix.data[9], 6, "decimals");
  }
});

test("payments: no app transfer when the app share is zero, none at all when nothing is owed", () => {
  const user = wallet().publicKey;
  const mint = wallet().publicKey;
  const anonymous = buildPaymentInstructions({ user, mint, noirwireAccount: wallet().publicKey, appAccount: null, split: { networkCost: 700n, markup: 700n, noirwire: 700n, app: 0n, total: 1400n } });
  assert.equal(anonymous.length, 1);
  assert.equal(anonymous[0].data.readBigUInt64LE(1), 1400n);
  const free = buildPaymentInstructions({ user, mint, noirwireAccount: wallet().publicKey, appAccount: null, split: { networkCost: 0n, markup: 0n, noirwire: 0n, app: 0n, total: 0n } });
  assert.equal(free.length, 0);
});

test("appending payments keeps a legacy message's payer, blockhash and instructions", () => {
  const payer = wallet().publicKey;
  const user = wallet().publicKey;
  const mint = wallet().publicKey;
  const original = buildMessage({ payer, legacy: true, instructions: [SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 7 })] });
  const final = appendInstructions(original, [], paymentsFor(user, mint));
  assert.equal(final.version, "legacy");
  assert.equal(final.recentBlockhash, original.recentBlockhash);
  assert.ok(final.staticAccountKeys[0].equals(payer));
  assert.equal(final.compiledInstructions.length, 3);
  const desc = describeMessage(final, []);
  assert.equal(desc.instructions[0].programId, SystemProgram.programId.toBase58());
  assert.equal(desc.instructions[1].accounts[3].isSigner, true);
});

test("appending payments to a v0 message keeps its lookup tables working", () => {
  const payer = wallet().publicKey;
  const user = wallet().publicKey;
  const mint = wallet().publicKey;
  const target = wallet().publicKey;
  const program = wallet().publicKey;
  const table = lookupTable([target, program, wallet().publicKey]);
  const original = buildMessage({
    payer,
    tables: [table],
    instructions: [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: target }])],
  });
  assert.equal(original.addressTableLookups.length, 1);
  const final = appendInstructions(original, [table], paymentsFor(user, mint));
  assert.equal(final.version, 0);
  assert.equal(final.recentBlockhash, original.recentBlockhash);
  assert.equal(final.addressTableLookups.length, 1, "the table is still used");
  assert.ok(final.staticAccountKeys[0].equals(payer));

  const before = describeMessage(original, [table]);
  const after = describeMessage(final, [table]);
  assert.deepEqual(after.instructions[0].accounts.map((a) => a.pubkey), before.instructions[0].accounts.map((a) => a.pubkey));
  assert.equal(after.instructions.length, 3);
  // The result decompiles again with the same table: nothing was lost on the way.
  assert.equal(TransactionMessage.decompile(final, { addressLookupTableAccounts: [table] }).instructions.length, 3);
});
