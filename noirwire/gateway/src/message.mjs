import { createHash } from "node:crypto";
import { ed25519 } from "@noble/curves/ed25519";
import { PublicKey, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { Refusal } from "./errors.mjs";

const BASE64 = /^[A-Za-z0-9+/]+={0,2}$/;

export function decodeBase64(text, what) {
  if (typeof text !== "string" || text.length === 0 || !BASE64.test(text)) {
    throw new Refusal("bad_transaction", `${what} must be base64`);
  }
  return Buffer.from(text, "base64");
}

export function decodeTransaction(text, maxBytes) {
  const raw = decodeBase64(text, "transaction");
  if (raw.length > maxBytes) throw new Refusal("too_large", `transaction is ${raw.length} bytes, the limit is ${maxBytes}`);
  try {
    return VersionedTransaction.deserialize(raw);
  } catch {
    throw new Refusal("bad_transaction", "transaction could not be decoded");
  }
}

export const messageBytes = (message) => Uint8Array.from(message.serialize());

/** The identity of a quote: sha256 of the exact bytes the user signs. */
export const messageHash = (message) => createHash("sha256").update(messageBytes(message)).digest("hex");

/**
 * Fetches and checks every address lookup table a v0 message uses. A table that is being
 * deactivated could be closed between now and landing, so only fully active ones are accepted.
 */
export async function resolveLookupTables(chain, message) {
  const lookups = message.addressTableLookups ?? [];
  const tables = [];
  for (const lookup of lookups) {
    const table = await chain.getAddressLookupTable(lookup.accountKey);
    if (!table) throw new Refusal("lookup_table_missing", `address lookup table ${lookup.accountKey.toBase58()} does not exist`);
    if (!table.isActive()) {
      throw new Refusal("lookup_table_inactive", `address lookup table ${lookup.accountKey.toBase58()} is deactivated or deactivating`);
    }
    tables.push(table);
  }
  return tables;
}

/**
 * The message as plain data with every lookup table resolved: all account keys, which of them
 * are signers, and each instruction's accounts in order.
 */
export function describeMessage(message, tables) {
  let keys;
  try {
    keys = message.getAccountKeys({ addressLookupTableAccounts: tables });
  } catch {
    throw new Refusal("bad_transaction", "the message references an address lookup table entry that does not exist");
  }
  const allKeys = [];
  for (let i = 0; i < keys.length; i += 1) allKeys.push(keys.get(i).toBase58());
  const staticKeys = message.staticAccountKeys.map((key) => key.toBase58());
  const numRequiredSignatures = message.header.numRequiredSignatures;
  const account = (index) => {
    if (index >= allKeys.length) throw new Refusal("bad_transaction", "an instruction references an account that does not exist");
    return { pubkey: allKeys[index], isSigner: index < numRequiredSignatures, isWritable: message.isAccountWritable(index) };
  };
  const instructions = message.compiledInstructions.map((ix) => {
    if (ix.programIdIndex >= allKeys.length) throw new Refusal("bad_transaction", "an instruction references a program that does not exist");
    return { programId: allKeys[ix.programIdIndex], accounts: ix.accountKeyIndexes.map(account), data: Uint8Array.from(ix.data) };
  });
  return { staticKeys, loadedKeys: allKeys.slice(staticKeys.length), allKeys, numRequiredSignatures, instructions };
}

const toInstruction = (ix) => ({
  programId: new PublicKey(ix.programId),
  keys: ix.accounts.map((a) => ({ pubkey: new PublicKey(a.pubkey), isSigner: a.isSigner, isWritable: a.isWritable })),
  data: Buffer.from(ix.data),
});

/**
 * The same message with `extra` instructions added at the end. The blockhash, fee payer and
 * lookup tables are kept, so a v0 message that relied on its tables still fits.
 */
export function appendInstructions(message, tables, extra) {
  const decompiled = TransactionMessage.decompile(message, { addressLookupTableAccounts: tables });
  const rebuilt = new TransactionMessage({
    payerKey: decompiled.payerKey,
    recentBlockhash: decompiled.recentBlockhash,
    instructions: [...decompiled.instructions, ...extra],
  });
  return message.version === "legacy" ? rebuilt.compileToLegacyMessage() : rebuilt.compileToV0Message(tables);
}

const isUnsigned = (signature) => signature.every((byte) => byte === 0);

/**
 * True when every required signer except the fee payer (slot 0) has signed these exact bytes.
 * The relayer signs last, so slot 0 is allowed to be empty.
 */
export function userSignaturesValid(tx) {
  const message = tx.message;
  const required = message.header.numRequiredSignatures;
  if (tx.signatures.length !== required) return false;
  const bytes = messageBytes(message);
  for (let i = 1; i < required; i += 1) {
    const signature = tx.signatures[i];
    if (signature.length !== 64 || isUnsigned(signature)) return false;
    if (!verifyEd25519(signature, bytes, message.staticAccountKeys[i].toBytes())) return false;
  }
  return true;
}

export function verifyEd25519(signature, bytes, publicKey) {
  try {
    return ed25519.verify(signature, bytes, publicKey);
  } catch {
    return false;
  }
}
