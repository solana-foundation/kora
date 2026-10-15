import { SendTransactionError } from "@solana/web3.js";
import { bounded } from "./bounded.mjs";
import { UnknownOutcome } from "./errors.mjs";

const ACCOUNTS_PER_REQUEST = 100;

/**
 * The few chain operations the gateway needs, over a web3.js Connection with a wall-clock
 * bound on every call. Tests replace this whole object with a fake.
 */
export function createChain(connection) {
  const conn = bounded(connection);
  return {
    getAccountInfo: (key, commitment = "confirmed") => conn.getAccountInfo(key, commitment),

    async getMultipleAccountsInfo(keys) {
      const infos = [];
      for (let i = 0; i < keys.length; i += ACCOUNTS_PER_REQUEST) {
        infos.push(...(await conn.getMultipleAccountsInfo(keys.slice(i, i + ACCOUNTS_PER_REQUEST), "confirmed")));
      }
      return infos;
    },

    async getAddressLookupTable(key) {
      return (await conn.getAddressLookupTable(key, { commitment: "confirmed" })).value;
    },

    async getBalance(key) {
      return BigInt(await conn.getBalance(key, "confirmed"));
    },

    /** Runs the transaction as it stands, unsigned, and reads back the fee payer's lamports afterwards. */
    async simulate(tx, feePayer) {
      const { value } = await conn.simulateTransaction(tx, {
        sigVerify: false,
        replaceRecentBlockhash: false,
        commitment: "confirmed",
        accounts: { encoding: "base64", addresses: [feePayer] },
      });
      const account = value.accounts?.[0];
      return { err: value.err, feePayerLamports: account ? BigInt(account.lamports) : null };
    },

    async isBlockhashValid(blockhash) {
      return (await conn.isBlockhashValid(blockhash, { commitment: "confirmed" })).value;
    },

    /**
     * Broadcasts with the node's preflight on. An error answer from the node means the
     * transaction was not accepted. Anything else (a timeout, a dropped connection) says
     * nothing about whether the node took it.
     */
    async sendRawTransaction(raw) {
      try {
        return await conn.sendRawTransaction(raw, { skipPreflight: false, preflightCommitment: "confirmed" });
      } catch (error) {
        if (error instanceof SendTransactionError) throw error;
        throw new UnknownOutcome(`the broadcast gave no clear answer: ${error.message}`);
      }
    },

    /** `{ confirmed, err }` once the cluster has the transaction, else null. */
    async getSignatureStatus(signature) {
      const { value } = await conn.getSignatureStatuses([signature], { searchTransactionHistory: true });
      const status = value[0];
      if (!status || !status.confirmationStatus || status.confirmationStatus === "processed") return null;
      return { err: status.err };
    },

    /** What the fee payer (always account 0) paid for a landed transaction, in lamports. */
    async getFeePayerLoss(signature) {
      const tx = await conn.getTransaction(signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
      if (!tx?.meta) return null;
      const lost = BigInt(tx.meta.preBalances[0]) - BigInt(tx.meta.postBalances[0]);
      return lost > 0n ? lost : 0n;
    },
  };
}
