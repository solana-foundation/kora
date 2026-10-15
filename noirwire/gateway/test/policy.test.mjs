import assert from "node:assert/strict";
import { test } from "node:test";
import { ComputeBudgetProgram, PublicKey, SystemProgram } from "@solana/web3.js";
import { createAssociatedTokenAccountIdempotentInstruction, createAssociatedTokenAccountInstruction, createInitializeAccount3Instruction } from "@solana/spl-token";
import { describeMessage, resolveLookupTables } from "../src/message.mjs";
import {
  baseFeeLamports,
  candidateTokenAccounts,
  checkAllowedPrograms,
  checkCaps,
  checkFeePayerExposure,
  checkFeePayerSlot,
  checkNoDurableNonce,
  checkSimulatedOutflow,
  checkUser,
  readComputeBudget,
  tokenAccountOwner,
} from "../src/policy.mjs";
import { accountInfo, buildMessage, FakeChain, lookupTable, programCall, TOKEN_PROGRAM, tokenAccountData, wallet } from "./helpers.mjs";

const caps = { maxSignatures: 3, maxComputeUnitPrice: 50_000, maxPriorityFeeLamports: 10_000, maxComputeUnitLimit: 1_400_000, maxFeePayerOutflowLamports: 2_200_000 };
const b58 = (key) => key.toBase58();

function setup() {
  const feePayer = wallet().publicKey;
  const user = wallet().publicKey;
  const program = wallet().publicKey;
  return { feePayer, user, program };
}

const describe = (payer, instructions, extra = {}) => describeMessage(buildMessage({ payer, instructions, ...extra }), extra.tables ?? []);
const exposure = (desc, feePayer, owned = new Set()) => checkFeePayerExposure(desc, b58(feePayer), feePayer.toBytes(), owned);

test("a program instruction that receives the fee payer is refused", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: feePayer, isWritable: true }])]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("a program instruction that does not touch the fee payer passes", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }])]);
  exposure(desc, feePayer);
});

test("the fee payer as a readonly, non-signer account is still refused", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: feePayer, isWritable: false }])]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("the fee payer cannot be the program id", () => {
  const { feePayer, user } = setup();
  const desc = describe(user, [programCall(feePayer, [{ pubkey: user, isSigner: true }])]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("a System transfer from the fee payer is refused", () => {
  const { feePayer, user } = setup();
  const desc = describe(feePayer, [SystemProgram.transfer({ fromPubkey: feePayer, toPubkey: user, lamports: 5 })]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("a System CreateAccount funded by the fee payer is allowed", () => {
  const { feePayer, program } = setup();
  const created = wallet();
  const desc = describe(feePayer, [
    SystemProgram.createAccount({ fromPubkey: feePayer, newAccountPubkey: created.publicKey, lamports: 1_000_000, space: 10, programId: program }),
  ]);
  exposure(desc, feePayer);
});

test("a CreateAccount that creates the fee payer's own address is refused", () => {
  const { feePayer, program } = setup();
  const other = wallet().publicKey;
  const desc = describe(other, [
    SystemProgram.createAccount({ fromPubkey: other, newAccountPubkey: feePayer, lamports: 1, space: 0, programId: program }),
  ]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("the fee payer in a CreateAccount in the new-account slot is refused even with another funder", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(user, [
    SystemProgram.createAccount({ fromPubkey: user, newAccountPubkey: feePayer, lamports: 1, space: 0, programId: program }),
  ]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("CreateAccountWithSeed, Assign and Allocate by the fee payer are refused", () => {
  const { feePayer, user, program } = setup();
  const seeded = describe(feePayer, [
    SystemProgram.createAccountWithSeed({ fromPubkey: feePayer, basePubkey: feePayer, seed: "s", newAccountPubkey: user, lamports: 1, space: 0, programId: program }),
  ]);
  assert.throws(() => exposure(seeded, feePayer), { code: "fee_payer_exposed" });
  const assign = describe(feePayer, [SystemProgram.assign({ accountPubkey: feePayer, programId: program })]);
  assert.throws(() => exposure(assign, feePayer), { code: "fee_payer_exposed" });
  const allocate = describe(feePayer, [SystemProgram.allocate({ accountPubkey: feePayer, space: 10 })]);
  assert.throws(() => exposure(allocate, feePayer), { code: "fee_payer_exposed" });
});

test("an associated token account created and funded by the fee payer is allowed", () => {
  const { feePayer, user } = setup();
  const mint = wallet().publicKey;
  const ata = wallet().publicKey;
  for (const create of [createAssociatedTokenAccountInstruction, createAssociatedTokenAccountIdempotentInstruction]) {
    const desc = describe(feePayer, [create(feePayer, ata, user, mint)]);
    exposure(desc, feePayer);
  }
});

test("an associated token account whose wallet is the fee payer is refused", () => {
  const { feePayer } = setup();
  const mint = wallet().publicKey;
  const ata = wallet().publicKey;
  const desc = describe(feePayer, [createAssociatedTokenAccountInstruction(feePayer, ata, feePayer, mint)]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("the fee payer is not allowed in the other positions of an associated token account instruction", () => {
  const { feePayer, user } = setup();
  const mint = wallet().publicKey;
  const desc = describe(user, [createAssociatedTokenAccountInstruction(user, feePayer, user, mint)]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("an associated token program instruction that is neither create nor create-idempotent gets no exception", () => {
  const { feePayer, user } = setup();
  const mint = wallet().publicKey;
  const ata = wallet().publicKey;
  const create = createAssociatedTokenAccountInstruction(feePayer, ata, user, mint);
  create.data = Buffer.from([2]); // RecoverNested
  const desc = describe(feePayer, [create]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("a token account owned by the fee payer is refused wherever it appears", () => {
  const { feePayer, user, program } = setup();
  const owned = wallet().publicKey;
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: owned }])]);
  exposure(desc, feePayer);
  assert.throws(() => exposure(desc, feePayer, new Set([b58(owned)])), { code: "fee_payer_exposed" });
});

test("a token account owned by the fee payer cannot be the funder or the created account of an allowed instruction", () => {
  const { feePayer, user } = setup();
  const owned = wallet().publicKey;
  const mint = wallet().publicKey;
  const desc = describe(feePayer, [createAssociatedTokenAccountInstruction(feePayer, owned, user, mint)]);
  assert.throws(() => exposure(desc, feePayer, new Set([b58(owned)])), { code: "fee_payer_exposed" });
});

test("initialising a new token account with the fee payer as owner is refused", () => {
  const { feePayer, user } = setup();
  const account = wallet().publicKey;
  const mint = wallet().publicKey;
  const desc = describe(user, [createInitializeAccount3Instruction(account, mint, feePayer, TOKEN_PROGRAM)]);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("exposure is found through an address lookup table", async () => {
  const { feePayer, user, program } = setup();
  const table = lookupTable([feePayer, wallet().publicKey]);
  const chain = new FakeChain();
  chain.tables.set(b58(table.key), table);
  const message = buildMessage({
    payer: user,
    instructions: [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: feePayer }])],
    tables: [table],
  });
  assert.equal(message.addressTableLookups.length, 1);
  assert.equal(message.staticAccountKeys.some((key) => key.equals(feePayer)), false);
  const resolved = await resolveLookupTables(chain, message);
  const desc = describeMessage(message, resolved);
  assert.throws(() => exposure(desc, feePayer), { code: "fee_payer_exposed" });
});

test("a deactivated or deactivating lookup table is refused", async () => {
  const { user, program } = setup();
  const table = lookupTable([wallet().publicKey], { active: false });
  const chain = new FakeChain();
  chain.tables.set(b58(table.key), table);
  const message = buildMessage({
    payer: user,
    instructions: [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: table.state.addresses[0] }])],
    tables: [table],
  });
  await assert.rejects(resolveLookupTables(chain, message), { code: "lookup_table_inactive" });
});

test("a lookup table that does not exist is refused", async () => {
  const { user, program } = setup();
  const table = lookupTable([wallet().publicKey]);
  const message = buildMessage({
    payer: user,
    instructions: [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: table.state.addresses[0] }])],
    tables: [table],
  });
  await assert.rejects(resolveLookupTables(new FakeChain(), message), { code: "lookup_table_missing" });
});

test("the fee payer must be the message fee payer", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(user, [programCall(program, [{ pubkey: user, isSigner: true }])]);
  assert.throws(() => checkFeePayerSlot(desc, b58(feePayer)), { code: "wrong_fee_payer" });
});

test("the fee payer as a signer outside slot 0 is refused", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(user, [programCall(program, [{ pubkey: feePayer, isSigner: true }])]);
  assert.throws(() => checkFeePayerSlot(desc, b58(feePayer)), { code: "wrong_fee_payer" });
});

test("the fee payer address duplicated through a lookup table is refused", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }])]);
  checkFeePayerSlot(desc, b58(feePayer));
  const duplicated = { ...desc, allKeys: [...desc.allKeys, b58(feePayer)] };
  assert.throws(() => checkFeePayerSlot(duplicated, b58(feePayer)), { code: "fee_payer_misuse" });
});

test("the user must be a required signer other than the fee payer", () => {
  const { feePayer, user, program } = setup();
  const stranger = wallet().publicKey;
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: stranger }])]);
  checkUser(desc, b58(user), b58(feePayer));
  assert.throws(() => checkUser(desc, b58(feePayer), b58(feePayer)), { code: "user_not_signer" });
  assert.throws(() => checkUser(desc, b58(stranger), b58(feePayer)), { code: "user_not_signer" });
  assert.throws(() => checkUser(desc, b58(wallet().publicKey), b58(feePayer)), { code: "user_not_signer" });
});

test("a durable nonce transaction is refused", () => {
  const { feePayer, user } = setup();
  const nonceAccount = wallet().publicKey;
  const advance = SystemProgram.nonceAdvance({ noncePubkey: nonceAccount, authorizedPubkey: user });
  const desc = describe(feePayer, [advance, SystemProgram.transfer({ fromPubkey: user, toPubkey: nonceAccount, lamports: 1 })]);
  assert.throws(() => checkNoDurableNonce(desc), { code: "durable_nonce" });
  const plain = describe(feePayer, [SystemProgram.transfer({ fromPubkey: user, toPubkey: nonceAccount, lamports: 1 })]);
  checkNoDurableNonce(plain);
});

test("compute budget: explicit limit and price give the priority fee", () => {
  const { feePayer, user } = setup();
  const desc = describe(feePayer, [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 10_000 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ]);
  const budget = readComputeBudget(desc);
  assert.equal(budget.unitLimit, 200_000);
  assert.equal(budget.unitPrice, 10_000n);
  // 10_000 micro-lamports x 200_000 units = 2_000 lamports; two signatures (payer and user)
  assert.deepEqual(baseFeeLamports(desc, budget), { signatureFee: 10_000n, priorityFee: 2_000n });
});

test("compute budget: without a limit, Solana's default of 200000 per instruction applies, and costs nothing at price zero", () => {
  const { feePayer, user } = setup();
  const transfers = [
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ];
  const budget = readComputeBudget(describe(feePayer, transfers));
  assert.equal(budget.unitLimit, null);
  assert.equal(budget.effectiveLimit, 400_000);
  assert.equal(baseFeeLamports(describe(feePayer, transfers), budget).priorityFee, 0n);
  const explicitZero = describe(feePayer, [ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 0 }), ...transfers]);
  assert.equal(readComputeBudget(explicitZero).unitPrice, 0n);
});

test("compute budget: a unit price without a unit limit is refused, because payments are appended afterwards", () => {
  const { feePayer, user } = setup();
  const desc = describe(feePayer, [
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 5_000 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ]);
  assert.throws(() => readComputeBudget(desc), { code: "compute_limit_required" });
});

test("compute budget: the fee measured before the payment is appended is the fee of the final message", () => {
  const { feePayer, user } = setup();
  const instructions = [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 150_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 4_000 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ];
  const before = describe(feePayer, instructions);
  const payments = [programCall(wallet().publicKey, [{ pubkey: user, isSigner: true }]), programCall(wallet().publicKey, [{ pubkey: user, isSigner: true }])];
  const after = describe(feePayer, [...instructions, ...payments]);
  assert.deepEqual(baseFeeLamports(before, readComputeBudget(before)), baseFeeLamports(after, readComputeBudget(after)));
});

test("allowed programs: an empty list allows everything, a list allows only itself plus the basics", () => {
  const { feePayer, user, program } = setup();
  const other = wallet().publicKey;
  const basics = [SystemProgram.transfer({ fromPubkey: user, toPubkey: other, lamports: 1 }), ComputeBudgetProgram.setComputeUnitLimit({ units: 1000 })];
  const calls = describe(feePayer, [...basics, programCall(program, [{ pubkey: user, isSigner: true }])]);
  checkAllowedPrograms(calls, []);
  checkAllowedPrograms(calls, [b58(program)]);
  assert.throws(() => checkAllowedPrograms(calls, [b58(other)]), { code: "program_not_allowed" });
  checkAllowedPrograms(describe(feePayer, basics), [b58(other)]);
});

test("compute budget: the default is capped at 1.4 million units and a missing price is free", () => {
  const { feePayer, user } = setup();
  const transfers = Array.from({ length: 8 }, () => SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }));
  const budget = readComputeBudget(describe(feePayer, transfers));
  assert.equal(budget.effectiveLimit, 1_400_000);
  assert.equal(budget.unitPrice, 0n);
});

test("compute budget: priority fee rounds up", () => {
  const { feePayer, user } = setup();
  const desc = describe(feePayer, [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ]);
  assert.equal(baseFeeLamports(desc, readComputeBudget(desc)).priorityFee, 1n);
});

test("compute budget: a repeated setting is refused", () => {
  const { feePayer, user } = setup();
  const desc = describe(feePayer, [
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 2 }),
    SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 }),
  ]);
  assert.throws(() => readComputeBudget(desc), { code: "bad_compute_budget" });
});

test("caps: signatures, compute unit price and limit", () => {
  const { feePayer, user } = setup();
  const send = SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 });
  const within = describe(feePayer, [ComputeBudgetProgram.setComputeUnitLimit({ units: 100_000 }), send]);
  checkCaps(within, readComputeBudget(within), caps);

  const expensive = describe(feePayer, [ComputeBudgetProgram.setComputeUnitLimit({ units: 1000 }), ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_001 }), send]);
  assert.throws(() => checkCaps(expensive, readComputeBudget(expensive), caps), { code: "priority_fee_too_high" });

  const heavy = describe(feePayer, [ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }), send]);
  assert.throws(() => checkCaps(heavy, readComputeBudget(heavy), { ...caps, maxComputeUnitLimit: 500_000 }), { code: "compute_limit_too_high" });

  const many = describe(feePayer, [programCall(wallet().publicKey, [1, 2, 3].map(() => ({ pubkey: wallet().publicKey, isSigner: true })))]);
  assert.equal(many.numRequiredSignatures, 4);
  assert.throws(() => checkCaps(many, readComputeBudget(many), caps), { code: "too_many_signatures" });
});

test("caps: the total priority fee per transaction has its own ceiling, whatever the price cap says", () => {
  const { feePayer, user } = setup();
  const send = SystemProgram.transfer({ fromPubkey: user, toPubkey: wallet().publicKey, lamports: 1 });
  const budgetFor = (units, microLamports) =>
    describe(feePayer, [ComputeBudgetProgram.setComputeUnitLimit({ units }), ComputeBudgetProgram.setComputeUnitPrice({ microLamports }), send]);
  const atCeiling = budgetFor(200_000, 50_000); // 10,000 lamports
  checkCaps(atCeiling, readComputeBudget(atCeiling), caps);
  const over = budgetFor(200_001, 50_000); // 10,001 lamports, price and limit each within their own caps
  assert.throws(() => checkCaps(over, readComputeBudget(over), caps), { code: "priority_fee_too_high" });
  const none = describe(feePayer, [send]);
  checkCaps(none, readComputeBudget(none), { ...caps, maxPriorityFeeLamports: 0 });
  const tiny = budgetFor(1000, 1);
  assert.throws(() => checkCaps(tiny, readComputeBudget(tiny), { ...caps, maxPriorityFeeLamports: 0 }), { code: "priority_fee_too_high" });
});

test("caps: the simulated fee payer outflow", () => {
  checkSimulatedOutflow(2_200_000n, caps);
  assert.throws(() => checkSimulatedOutflow(2_200_001n, caps), { code: "outflow_too_high" });
});

test("only accounts that are token accounts are looked up, and only owners are read from them", () => {
  const { feePayer, user, program } = setup();
  const desc = describe(feePayer, [programCall(program, [{ pubkey: user, isSigner: true }, { pubkey: feePayer }])]);
  assert.ok(!candidateTokenAccounts(desc, b58(feePayer)).includes(b58(feePayer)));

  const owner = wallet().publicKey;
  const info = accountInfo(TOKEN_PROGRAM, tokenAccountData({ mint: wallet().publicKey, owner }));
  assert.ok(tokenAccountOwner(info).equals(owner.toBuffer()));
  assert.equal(tokenAccountOwner(null), null);
  assert.equal(tokenAccountOwner(accountInfo(SystemProgram.programId, Buffer.alloc(165))), null);
  assert.equal(tokenAccountOwner(accountInfo(TOKEN_PROGRAM, Buffer.alloc(82))), null);
  assert.equal(tokenAccountOwner(accountInfo(new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"), Buffer.alloc(170))), null);
});
