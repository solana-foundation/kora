import { KoraClient } from "../src/index.js";
import setupTestSuit from "./setup.js";

import {
  Address,
  getBase64EncodedWireTransaction,
  getBase64Encoder,
  getTransactionDecoder,
  signTransaction,
  type KeyPairSigner,
  type Transaction,
} from "@solana/kit";

function transactionFromBase64(base64: string): Transaction {
  const encoder = getBase64Encoder();
  const decoder = getTransactionDecoder();
  const messageBytes = encoder.encode(base64);
  return decoder.decode(messageBytes);
}

describe("KoraClient Integration Tests", () => {
  let client: KoraClient;
  let testWallet: KeyPairSigner;
  let testWalletAddress: Address;
  let destinationAddress: Address;
  let usdcMint: Address;
  let koraAddress: Address;

  beforeAll(async () => {
    const testSuite = await setupTestSuit();
    client = testSuite.koraClient;
    testWallet = testSuite.testWallet;
    testWalletAddress = testWallet.address;
    destinationAddress = testSuite.destinationAddress;
    usdcMint = testSuite.usdcMint;
    koraAddress = testSuite.koraAddress;
  }, 30000); // allow adequte time for airdrops and token initialization

  describe("Configuration and Setup", () => {
    it("should get config", async () => {
      const config = await client.getConfig();
      expect(config).toBeDefined();
      expect(config.fee_payer).toBeDefined();
      expect(config.validation_config).toBeDefined();
      expect(config.validation_config.allowed_programs).toBeDefined();
      expect(config.validation_config.allowed_tokens).toBeDefined();
      expect(config.validation_config.max_allowed_lamports).toBeDefined();
      expect(config.validation_config.max_signatures).toBeDefined();
      expect(config.validation_config.price_source).toBeDefined();
    });

    it("should get supported tokens", async () => {
      const { tokens } = await client.getSupportedTokens();
      expect(Array.isArray(tokens)).toBe(true);
      expect(tokens.length).toBeGreaterThan(0);
      expect(tokens).toContain(usdcMint); // USDC should be supported
    });

    it("should get blockhash", async () => {
      const { blockhash } = await client.getBlockhash();
      expect(blockhash).toBeDefined();
      expect(typeof blockhash).toBe("string");
      expect(blockhash.length).toBeGreaterThanOrEqual(43);
      expect(blockhash.length).toBeLessThanOrEqual(44); // Base58 encoded hash length
    });
  });

  describe("Transaction Operations", () => {
    it("should create transfer transaction", async () => {
      const request = {
        amount: 1000000, // 1 USDC
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      const response = await client.transferTransaction(request);
      expect(response).toBeDefined();
      expect(response.transaction).toBeDefined();
      expect(response.blockhash).toBeDefined();
      expect(response.message).toBeDefined();
    });

    it("should estimate transaction fee", async () => {
      // First create a transaction
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: testWalletAddress,
      };

      const { transaction } = await client.transferTransaction(transferRequest);
      const fee = await client.estimateTransactionFee(transaction, usdcMint);

      expect(fee).toBeDefined();
      expect(typeof fee.fee_in_lamports).toBe("number");
      expect(fee.fee_in_lamports).toBeGreaterThan(0);
    });

    it("should sign transaction", async () => {
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      const { transaction } = await client.transferTransaction(transferRequest);
      const signResult = await client.signTransaction({ transaction });

      expect(signResult).toBeDefined();
      expect(signResult.signature).toBeDefined();
      expect(signResult.signed_transaction).toBeDefined();
    });

    it("should sign and send transaction", async () => {
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      const { transaction: transactionString } =
        await client.transferTransaction(transferRequest);
      const transaction = transactionFromBase64(transactionString);

      // Sign transaction with test wallet before sending
      const signedTransaction = await signTransaction(
        [testWallet.keyPair],
        transaction,
      );
      const base64SignedTransaction =
        getBase64EncodedWireTransaction(signedTransaction);
      const signResult = await client.signAndSendTransaction({
        transaction: base64SignedTransaction,
      });

      expect(signResult).toBeDefined();
      expect(signResult.signature).toBeDefined();
      expect(signResult.signed_transaction).toBeDefined();
    });

    it("should sign transaction if paid", async () => {
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      const { transaction } = await client.transferTransaction(transferRequest);

      const signResult = await client.signTransactionIfPaid({
        transaction,
      });

      expect(signResult).toBeDefined();
      expect(signResult.transaction).toBeDefined();
      expect(signResult.signed_transaction).toBeDefined();
    });
  });

  describe("Error Handling", () => {
    it("should handle invalid token address", async () => {
      const request = {
        amount: 1000000,
        token: "InvalidTokenAddress",
        source: testWalletAddress,
        destination: destinationAddress,
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });

    it("should handle invalid amount", async () => {
      const request = {
        amount: -1, // Invalid amount
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });

    it("should handle invalid transaction for signing", async () => {
      await expect(
        client.signTransaction({
          transaction: "invalid_transaction",
        }),
      ).rejects.toThrow();
    });

    it("should handle invalid transaction for fee estimation", async () => {
      await expect(
        client.estimateTransactionFee("invalid_transaction", usdcMint),
      ).rejects.toThrow();
    });

    it("should handle non-allowed token for fee payment", async () => {
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      // TODO: API has an error. this endpoint should verify the provided fee token is supported
      const { transaction } = await client.transferTransaction(transferRequest);
      const fee = await client.estimateTransactionFee(
        transaction,
        "InvalidTokenAddress",
      );
      expect(fee).toBeDefined();
      expect(typeof fee.fee_in_lamports).toBe("number");
      expect(fee.fee_in_lamports).toBeGreaterThan(0);
    });
  });

  describe("End-to-End Flows", () => {
    it("should handle transfer and sign flow", async () => {
      const request = {
        amount: 1000000,
        token: usdcMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      // Create and sign the transaction
      const { transaction } = await client.transferTransaction(request);
      const signResult = await client.signTransaction({ transaction });

      expect(signResult.signature).toBeDefined();
      expect(signResult.signed_transaction).toBeDefined();
    });

    it("should reject transaction with non-allowed token", async () => {
      const invalidTokenMint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // Mainnet USDC mint
      const request = {
        amount: 1000000,
        token: invalidTokenMint,
        source: testWalletAddress,
        destination: destinationAddress,
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });
  });
});
