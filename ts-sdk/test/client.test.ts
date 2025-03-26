import { convertToObject } from 'typescript';
import { KoraClient } from '../src';
import { PublicKey } from '@solana/web3.js';

describe('KoraClient Integration Tests', () => {
  let client: KoraClient;
  const rpcUrl = process.env.KORA_RPC_URL!;
  const testWallet = process.env.TEST_WALLET_PUBKEY!;
  const usdcMint = process.env.USDC_MINT!;

  beforeAll(() => {
    client = new KoraClient(rpcUrl);
  });

  describe('Configuration and Setup', () => {
    it('should get config', async () => {
      const config = await client.getConfig();
      expect(config).toBeDefined();
      expect(config.fee_payer).toBeDefined();
      expect(config.validation_config).toBeDefined();
    });

    it('should get supported tokens', async () => {
      const { tokens } = await client.getSupportedTokens();
      expect(Array.isArray(tokens)).toBe(true);
      expect(tokens.length).toBeGreaterThan(0);
      // USDC should be supported
      expect(tokens).toContain(usdcMint);
    });

    it('should get blockhash', async () => {
      const { blockhash } = await client.getBlockhash();
      expect(blockhash).toBeDefined();
      expect(typeof blockhash).toBe('string');
      expect(blockhash.length).toBeGreaterThanOrEqual(43);
      expect(blockhash.length).toBeLessThanOrEqual(44); // Base58 encoded hash length
    });
  });

  describe('Transaction Operations', () => {
    it('should create transfer transaction', async () => {
      const request = {
        amount: 1000000, // 1 USDC
        token: usdcMint,
        source: testWallet,
        destination: new PublicKey(testWallet).toBase58(), // Send to self for testing
      };

      const response = await client.transferTransaction(request);
      expect(response).toBeDefined();
      expect(response.transaction).toBeDefined();
      expect(response.blockhash).toBeDefined();
      expect(response.message).toBeDefined();
    });

    it('should estimate transaction fee', async () => {
      // First create a transaction
      const transferRequest = {
        amount: 1000000,
        token: usdcMint,
        source: testWallet,
        destination: new PublicKey(testWallet).toBase58(),
      };

      const { transaction } = await client.transferTransaction(transferRequest);
      const fee = await client.estimateTransactionFee(transaction, usdcMint);
      
      expect(fee).toBeDefined();
      expect(typeof fee.fee_in_lamports).toBe('number');
      expect(fee.fee_in_lamports).toBeGreaterThan(0);
    });
  });

  describe('Error Handling', () => {
    it('should handle invalid token address', async () => {
      const request = {
        amount: 1000000,
        token: 'InvalidTokenAddress',
        source: testWallet,
        destination: new PublicKey(testWallet).toBase58(),
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });

    it('should handle invalid amount', async () => {
      const request = {
        amount: -1, // Invalid amount
        token: usdcMint,
        source: testWallet,
        destination: new PublicKey(testWallet).toBase58(),
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });
  });

  describe('End-to-End Flows', () => {
    it('should handle transfer and sign flow', async () => {
      // This test might fail without proper signing authority
      const request = {
        amount: 1000000,
        token: usdcMint,
        source: testWallet,
        destination: new PublicKey(testWallet).toBase58(),
      };

      // Create and sign the transaction
      const { transaction } = await client.transferTransaction(request);
      const signResult = await client.signTransaction({ transaction });

      expect(signResult.signature).toBeDefined();
      expect(signResult.signed_transaction).toBeDefined();
    });

    it('should reject transaction with non-allowed token', async () => {
      const invalidTokenMint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'; // Mainnet USDC mint
      const request = {
        amount: 1000000,
        token: invalidTokenMint,
        source: testWallet, 
        destination: new PublicKey(testWallet).toBase58(),
      };

      await expect(client.transferTransaction(request)).rejects.toThrow();
    });
  });
}); 