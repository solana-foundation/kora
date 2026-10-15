import { Refusal } from "./errors.mjs";
import { ceilDiv } from "./units.mjs";

// Pure checks over a described message (see describeMessage). Each throws a Refusal; none
// touches the network.

export const SYSTEM_PROGRAM = "11111111111111111111111111111111";
export const TOKEN_PROGRAM = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
export const TOKEN_2022_PROGRAM = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
export const ASSOCIATED_TOKEN_PROGRAM = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
export const COMPUTE_BUDGET_PROGRAM = "ComputeBudget111111111111111111111111111111";

export const LAMPORTS_PER_SIGNATURE = 5_000n;
export const MAX_COMPUTE_UNITS = 1_400_000;
const DEFAULT_UNITS_PER_INSTRUCTION = 200_000;
const MICRO_LAMPORTS_PER_LAMPORT = 1_000_000n;

const SYSTEM_CREATE_ACCOUNT = 0;
const SYSTEM_ADVANCE_NONCE = 4;
const TOKEN_INITIALIZE_ACCOUNT_2 = 16;
const TOKEN_INITIALIZE_ACCOUNT_3 = 18;
const ATA_CREATE = 0;
const ATA_CREATE_IDEMPOTENT = 1;

const systemDiscriminant = (ix) => (ix.programId === SYSTEM_PROGRAM && ix.data.length >= 4 ? Buffer.from(ix.data).readUInt32LE(0) : null);

/** The fee payer must be the message's own fee payer, and appear nowhere else in it. */
export function checkFeePayerSlot(desc, feePayer) {
  if (desc.staticKeys[0] !== feePayer) {
    throw new Refusal("wrong_fee_payer", `the transaction's fee payer must be ${feePayer}`);
  }
  if (desc.allKeys.filter((key) => key === feePayer).length !== 1) {
    throw new Refusal("fee_payer_misuse", "the fee payer address appears more than once in the transaction");
  }
}

/** The user pays in USDC, so they must sign; the fee payer cannot be the user. */
export function checkUser(desc, user, feePayer) {
  const index = desc.staticKeys.indexOf(user);
  if (user === feePayer || index < 1 || index >= desc.numRequiredSignatures) {
    throw new Refusal("user_not_signer", "user must be a required signer of the transaction other than the fee payer");
  }
}

/** Refuses a transaction that moves through a durable nonce. */
export function checkNoDurableNonce(desc) {
  if (desc.instructions.some((ix) => systemDiscriminant(ix) === SYSTEM_ADVANCE_NONCE)) {
    throw new Refusal("durable_nonce", "durable nonce transactions are not accepted");
  }
}

/**
 * Compute budget settings as the runtime reads them. Unknown or repeated compute budget
 * instructions are refused rather than guessed at, so the fee below is the fee charged.
 */
export function readComputeBudget(desc) {
  let unitPrice = null;
  let unitLimit = null;
  let budgetInstructions = 0;
  for (const ix of desc.instructions) {
    if (ix.programId !== COMPUTE_BUDGET_PROGRAM) continue;
    budgetInstructions += 1;
    const data = Buffer.from(ix.data);
    const kind = data[0];
    if (kind === 2 && data.length === 5) {
      if (unitLimit !== null) throw new Refusal("bad_compute_budget", "compute unit limit is set twice");
      unitLimit = data.readUInt32LE(1);
    } else if (kind === 3 && data.length === 9) {
      if (unitPrice !== null) throw new Refusal("bad_compute_budget", "compute unit price is set twice");
      unitPrice = data.readBigUInt64LE(1);
    } else if (!((kind === 1 || kind === 4) && data.length === 5)) {
      throw new Refusal("bad_compute_budget", "unsupported compute budget instruction");
    }
  }
  // Without an explicit limit the runtime's default grows with the instruction count, and the
  // gateway adds payment instructions afterwards. A priced transaction must therefore state its
  // limit, so the fee measured here is the fee the final message pays.
  if (unitPrice !== null && unitPrice > 0n && unitLimit === null) {
    throw new Refusal("compute_limit_required", "a transaction with a compute unit price must also set a compute unit limit");
  }
  const otherInstructions = desc.instructions.length - budgetInstructions;
  const effectiveLimit = Math.min(unitLimit ?? DEFAULT_UNITS_PER_INSTRUCTION * otherInstructions, MAX_COMPUTE_UNITS);
  return { unitPrice: unitPrice ?? 0n, unitLimit, effectiveLimit };
}

/** Signature fees plus the priority fee the compute budget buys, in lamports. */
export function baseFeeLamports(desc, budget) {
  const signatureFee = LAMPORTS_PER_SIGNATURE * BigInt(desc.numRequiredSignatures);
  const priorityFee = ceilDiv(budget.unitPrice * BigInt(budget.effectiveLimit), MICRO_LAMPORTS_PER_LAMPORT);
  return { signatureFee, priorityFee };
}

export function checkCaps(desc, budget, caps) {
  if (desc.numRequiredSignatures > caps.maxSignatures) {
    throw new Refusal("too_many_signatures", `at most ${caps.maxSignatures} signatures are allowed`);
  }
  if (budget.unitPrice > BigInt(caps.maxComputeUnitPrice)) {
    throw new Refusal("priority_fee_too_high", `compute unit price is above ${caps.maxComputeUnitPrice} micro-lamports`);
  }
  const { priorityFee } = baseFeeLamports(desc, budget);
  if (priorityFee > BigInt(caps.maxPriorityFeeLamports)) {
    throw new Refusal("priority_fee_too_high", `the priority fee is ${priorityFee} lamports, above the ${caps.maxPriorityFeeLamports} allowed per transaction`);
  }
  if (budget.effectiveLimit > caps.maxComputeUnitLimit) {
    throw new Refusal(
      "compute_limit_too_high",
      `compute unit limit is above ${caps.maxComputeUnitLimit}; set an explicit SetComputeUnitLimit within it`,
    );
  }
}

const ALWAYS_ALLOWED_PROGRAMS = new Set([SYSTEM_PROGRAM, TOKEN_PROGRAM, TOKEN_2022_PROGRAM, ASSOCIATED_TOKEN_PROGRAM, COMPUTE_BUDGET_PROGRAM]);

/** An app that lists programs only relays calls to those, plus the system, token and compute budget basics. */
export function checkAllowedPrograms(desc, allowedPrograms) {
  if (allowedPrograms.length === 0) return;
  const allowed = new Set(allowedPrograms);
  for (const ix of desc.instructions) {
    if (!ALWAYS_ALLOWED_PROGRAMS.has(ix.programId) && !allowed.has(ix.programId)) {
      throw new Refusal("program_not_allowed", `program ${ix.programId} is not allowed for this app`);
    }
  }
}

export function checkSimulatedOutflow(lamportLoss, caps) {
  if (lamportLoss > BigInt(caps.maxFeePayerOutflowLamports)) {
    throw new Refusal("outflow_too_high", `the transaction would cost the fee payer more than ${caps.maxFeePayerOutflowLamports} lamports`);
  }
}

const touchesFeePayer = (key, feePayer, ownedTokenAccounts) => key === feePayer || ownedTokenAccounts.has(key);

function isFundingInstruction(ix, feePayer, ownedTokenAccounts) {
  if (ix.accounts.length < 2 || ix.accounts[0].pubkey !== feePayer) return false;
  if (systemDiscriminant(ix) === SYSTEM_CREATE_ACCOUNT) {
    return !touchesFeePayer(ix.accounts[1].pubkey, feePayer, ownedTokenAccounts);
  }
  if (ix.programId === ASSOCIATED_TOKEN_PROGRAM) {
    const kind = ix.data.length === 0 ? ATA_CREATE : ix.data.length === 1 ? ix.data[0] : -1;
    if (kind !== ATA_CREATE && kind !== ATA_CREATE_IDEMPOTENT) return false;
    // funder, token account, wallet, mint, system program, token program
    return ix.accounts.length >= 6 && !touchesFeePayer(ix.accounts[1].pubkey, feePayer, ownedTokenAccounts) && ix.accounts[2].pubkey !== feePayer;
  }
  return false;
}

/** A token account initialised with the fee payer as owner, where the owner is in the data and not the accounts. */
function initialisesFeePayerOwnedAccount(ix, feePayerBytes) {
  if (ix.programId !== TOKEN_PROGRAM && ix.programId !== TOKEN_2022_PROGRAM) return false;
  const kind = ix.data[0];
  if ((kind !== TOKEN_INITIALIZE_ACCOUNT_2 && kind !== TOKEN_INITIALIZE_ACCOUNT_3) || ix.data.length < 33) return false;
  return Buffer.from(ix.data.subarray(1, 33)).equals(feePayerBytes);
}

/**
 * The fee payer is never handed to a program. Its address, and every token account it owns,
 * may appear in an instruction's accounts only as the funder of a top-level System
 * CreateAccount or Associated Token Account Create / CreateIdempotent. Anything else could let
 * a program spend the fee payer's SOL or tokens by CPI.
 *
 * `feePayerBytes` is the fee payer's 32 raw bytes.
 */
export function checkFeePayerExposure(desc, feePayer, feePayerBytes, ownedTokenAccounts) {
  for (const ix of desc.instructions) {
    if (touchesFeePayer(ix.programId, feePayer, ownedTokenAccounts)) {
      throw new Refusal("fee_payer_exposed", "the fee payer cannot be used as a program");
    }
    if (initialisesFeePayerOwnedAccount(ix, feePayerBytes)) {
      throw new Refusal("fee_payer_exposed", "a token account owned by the fee payer cannot be created");
    }
    ix.accounts.forEach((account, position) => {
      if (!touchesFeePayer(account.pubkey, feePayer, ownedTokenAccounts)) return;
      const allowed = position === 0 && account.pubkey === feePayer && isFundingInstruction(ix, feePayer, ownedTokenAccounts);
      if (!allowed) {
        throw new Refusal("fee_payer_exposed", "the fee payer, or a token account it owns, appears in an instruction where it is not allowed");
      }
    });
  }
}

/** Keys a transaction touches that could be token accounts, so their owners can be looked up. */
export function candidateTokenAccounts(desc, feePayer) {
  const keys = new Set();
  for (const ix of desc.instructions) {
    for (const account of ix.accounts) if (account.pubkey !== feePayer) keys.add(account.pubkey);
  }
  return [...keys];
}

const TOKEN_ACCOUNT_SIZE = 165;
const TOKEN_2022_ACCOUNT_TYPE = 2;

/** The owner of a token account (either token program), or null when the account is something else. */
export function tokenAccountOwner(info) {
  if (!info) return null;
  const program = info.owner.toBase58();
  if (program !== TOKEN_PROGRAM && program !== TOKEN_2022_PROGRAM) return null;
  const data = info.data;
  const isAccount =
    program === TOKEN_PROGRAM
      ? data.length === TOKEN_ACCOUNT_SIZE
      : data.length === TOKEN_ACCOUNT_SIZE || (data.length > TOKEN_ACCOUNT_SIZE && data[TOKEN_ACCOUNT_SIZE] === TOKEN_2022_ACCOUNT_TYPE);
  return isAccount ? Buffer.from(data.subarray(32, 64)) : null;
}
