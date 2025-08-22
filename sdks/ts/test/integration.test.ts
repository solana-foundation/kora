import { KoraClient } from '../src/index.js';
import setupTestSuite from './setup.js';
import { runAuthenticationTests } from './auth-setup.js';
import {
    Address,
    getBase64EncodedWireTransaction,
    getBase64Encoder,
    getTransactionDecoder,
    signTransaction,
    type KeyPairSigner,
    type Transaction,
} from '@solana/kit';
import { findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';

function transactionFromBase64(base64: string): Transaction {
    const encoder = getBase64Encoder();
    const decoder = getTransactionDecoder();
    const messageBytes = encoder.encode(base64);
    return decoder.decode(messageBytes);
}

const AUTH_ENABLED = process.env.ENABLE_AUTH === 'true';
const KORA_SIGNER_TYPE = process.env.KORA_SIGNER_TYPE || 'memory';
describe(`KoraClient Integration Tests (${AUTH_ENABLED ? 'with auth' : 'without auth'} | signer type: ${KORA_SIGNER_TYPE})`, () => {
    let client: KoraClient;
    let testWallet: KeyPairSigner;
    let testWalletAddress: Address;
    let destinationAddress: Address;
    let usdcMint: Address;
    let koraAddress: Address;

    beforeAll(async () => {
        const testSuite = await setupTestSuite();
        client = testSuite.koraClient;
        testWallet = testSuite.testWallet;
        testWalletAddress = testWallet.address;
        destinationAddress = testSuite.destinationAddress;
        usdcMint = testSuite.usdcMint;
        koraAddress = testSuite.koraAddress;
    }, 30000); // allow adequte time for airdrops and token initialization

    // Run authentication tests only when auth is enabled
    if (AUTH_ENABLED) {
        runAuthenticationTests('http://localhost:8080/');
    }

    describe('Configuration and Setup', () => {
        it('should get config', async () => {
            const config = await client.getConfig();
            expect(config).toBeDefined();
            expect(config.fee_payers).toBeDefined();
            expect(Array.isArray(config.fee_payers)).toBe(true);
            expect(config.fee_payers.length).toBeGreaterThan(0);
            expect(config.validation_config).toBeDefined();
            expect(config.validation_config.allowed_programs).toBeDefined();
            expect(config.validation_config.allowed_tokens).toBeDefined();
            expect(config.validation_config.max_allowed_lamports).toBeDefined();
            expect(config.validation_config.max_signatures).toBeDefined();
            expect(config.validation_config.price_source).toBeDefined();
            expect(config.validation_config.price).toBeDefined();
            expect(config.validation_config.price.type).toBeDefined();
            expect(config.validation_config.fee_payer_policy).toBeDefined();
            expect(config.validation_config.fee_payer_policy.allow_sol_transfers).toBeDefined();
            expect(config.validation_config.fee_payer_policy.allow_spl_transfers).toBeDefined();
            expect(config.validation_config.fee_payer_policy.allow_token2022_transfers).toBeDefined();
            expect(config.validation_config.fee_payer_policy.allow_assign).toBeDefined();
            expect(config.validation_config.fee_payer_policy.allow_burn).toBeDefined();
            expect(config.enabled_methods).toBeDefined();
            expect(config.enabled_methods.liveness).toBeDefined();
            expect(config.enabled_methods.estimate_transaction_fee).toBeDefined();
            expect(config.enabled_methods.get_supported_tokens).toBeDefined();
            expect(config.enabled_methods.sign_transaction).toBeDefined();
            expect(config.enabled_methods.sign_and_send_transaction).toBeDefined();
            expect(config.enabled_methods.transfer_transaction).toBeDefined();
            expect(config.enabled_methods.get_blockhash).toBeDefined();
            expect(config.enabled_methods.get_config).toBeDefined();
            expect(config.enabled_methods.sign_transaction_if_paid).toBeDefined();
        });

        it('should get payer signer', async () => {
            const { signer_address, payment_address } = await client.getPayerSigner();
            expect(signer_address).toBeDefined();
            expect(payment_address).toBeDefined();
        });

        it('should get supported tokens', async () => {
            const { tokens } = await client.getSupportedTokens();
            expect(Array.isArray(tokens)).toBe(true);
            expect(tokens.length).toBeGreaterThan(0);
            expect(tokens).toContain(usdcMint); // USDC should be supported
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
                source: testWalletAddress,
                destination: destinationAddress,
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
                source: testWalletAddress,
                destination: testWalletAddress,
            };

            const { transaction } = await client.transferTransaction(transferRequest);
            const fee = await client.estimateTransactionFee({ transaction, fee_token: usdcMint });

            expect(fee).toBeDefined();
            expect(typeof fee.fee_in_lamports).toBe('number');
            expect(fee.fee_in_lamports).toBeGreaterThan(0);
            expect(typeof fee.fee_in_token).toBe('number');
            expect(fee.fee_in_token).toBeGreaterThan(0);
        });

        it('should sign transaction', async () => {
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

        it('should sign and send transaction', async () => {
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: destinationAddress,
            };

            const { transaction: transactionString } = await client.transferTransaction(transferRequest);
            const transaction = transactionFromBase64(transactionString);
            // Sign transaction with test wallet before sending
            const signedTransaction = await signTransaction([testWallet.keyPair], transaction);
            const base64SignedTransaction = getBase64EncodedWireTransaction(signedTransaction);
            const signResult = await client.signAndSendTransaction({
                transaction: base64SignedTransaction,
            });

            expect(signResult).toBeDefined();
            expect(signResult.signature).toBeDefined();
            expect(signResult.signed_transaction).toBeDefined();
        });

        it('should sign transaction if paid', async () => {
            const config = await client.getConfig();
            const paymentAddress = config.fee_payers[0];
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: paymentAddress,
            };

            const { transaction } = await client.transferTransaction(transferRequest);

            const signResult = await client.signTransactionIfPaid({
                transaction,
            });

            expect(signResult).toBeDefined();
            expect(signResult.transaction).toBeDefined();
            expect(signResult.signed_transaction).toBeDefined();
        });

        it('should get payment instruction', async () => {
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: destinationAddress,
            };
            const [expectedSenderAta] = await findAssociatedTokenPda({
                owner: testWalletAddress,
                tokenProgram: TOKEN_PROGRAM_ADDRESS,
                mint: usdcMint,
            });
            const [koraAta] = await findAssociatedTokenPda({
                owner: koraAddress,
                tokenProgram: TOKEN_PROGRAM_ADDRESS,
                mint: usdcMint,
            });

            const { transaction } = await client.transferTransaction(transferRequest);
            const {
                payment_instruction,
                payment_amount,
                payment_token,
                payment_address,
                signer_address,
                original_transaction,
            } = await client.getPaymentInstruction({
                transaction,
                fee_token: usdcMint,
                source_wallet: testWalletAddress,
            });
            expect(payment_instruction).toBeDefined();
            expect(payment_instruction.programAddress).toBe(TOKEN_PROGRAM_ADDRESS);
            expect(payment_instruction.accounts?.[0].address).toBe(expectedSenderAta);
            expect(payment_instruction.accounts?.[1].address).toBe(koraAta);
            expect(payment_instruction.accounts?.[2].address).toBe(testWalletAddress);
            // todo math to verify payment amount
            // expect(payment_amount).toBe(1000000);
            expect(payment_token).toBe(usdcMint);
            expect(payment_address).toBe(koraAddress);
            expect(signer_address).toBe(koraAddress);
            expect(original_transaction).toBe(transaction);
        });
    });

    describe('Error Handling', () => {
        it('should handle invalid token address', async () => {
            const request = {
                amount: 1000000,
                token: 'InvalidTokenAddress',
                source: testWalletAddress,
                destination: destinationAddress,
            };

            await expect(client.transferTransaction(request)).rejects.toThrow();
        });

        it('should handle invalid amount', async () => {
            const request = {
                amount: -1, // Invalid amount
                token: usdcMint,
                source: testWalletAddress,
                destination: destinationAddress,
            };

            await expect(client.transferTransaction(request)).rejects.toThrow();
        });

        it('should handle invalid transaction for signing', async () => {
            await expect(
                client.signTransaction({
                    transaction: 'invalid_transaction',
                }),
            ).rejects.toThrow();
        });

        it('should handle invalid transaction for fee estimation', async () => {
            await expect(
                client.estimateTransactionFee({ transaction: 'invalid_transaction', fee_token: usdcMint }),
            ).rejects.toThrow();
        });

        it('should handle non-allowed token for fee payment', async () => {
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: destinationAddress,
            };

            // TODO: API has an error. this endpoint should verify the provided fee token is supported
            const { transaction } = await client.transferTransaction(transferRequest);
            const fee = await client.estimateTransactionFee({ transaction, fee_token: usdcMint });
            expect(fee).toBeDefined();
            expect(typeof fee.fee_in_lamports).toBe('number');
            expect(fee.fee_in_lamports).toBeGreaterThan(0);
        });
    });

    describe('End-to-End Flows', () => {
        it('should handle transfer and sign flow', async () => {
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

        it('should reject transaction with non-allowed token', async () => {
            const invalidTokenMint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'; // Mainnet USDC mint
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
