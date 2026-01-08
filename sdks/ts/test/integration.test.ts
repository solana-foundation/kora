import { KoraClient } from '../src/index.js';
import setupTestSuite from './setup.js';
import { runAuthenticationTests } from './auth-setup.js';
import {
    Address,
    getBase64Decoder,
    getBase64Encoder,
    getTransactionDecoder,
    getTransactionEncoder,
    partiallySignTransaction,
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

function transactionToBase64(transaction: Transaction): string {
    const txEncoder = getTransactionEncoder();
    const txBytes = txEncoder.encode(transaction);
    const base64Decoder = getBase64Decoder();
    return base64Decoder.decode(txBytes);
}

const AUTH_ENABLED = process.env.ENABLE_AUTH === 'true';
const KORA_SIGNER_TYPE = process.env.KORA_SIGNER_TYPE || 'memory';
describe(`KoraClient Integration Tests (${AUTH_ENABLED ? 'with auth' : 'without auth'} | signer type: ${KORA_SIGNER_TYPE})`, () => {
    let client: KoraClient;
    let testWallet: KeyPairSigner;
    let testWalletAddress: Address;
    let usdcMint: Address;
    let koraAddress: Address;

    beforeAll(async () => {
        const testSuite = await setupTestSuite();
        client = testSuite.koraClient;
        testWallet = testSuite.testWallet;
        testWalletAddress = testWallet.address;
        usdcMint = testSuite.usdcMint;
        koraAddress = testSuite.koraAddress;
    }, 90000); // allow adequate time for airdrops and token initialization

    // Run authentication tests only when auth is enabled
    if (AUTH_ENABLED) {
        runAuthenticationTests();
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

            // System policy
            expect(config.validation_config.fee_payer_policy.system).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.allow_transfer).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.allow_assign).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.allow_create_account).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.allow_allocate).toBeDefined();

            // System nonce policy
            expect(config.validation_config.fee_payer_policy.system.nonce).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.nonce.allow_initialize).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.nonce.allow_advance).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.nonce.allow_authorize).toBeDefined();
            expect(config.validation_config.fee_payer_policy.system.nonce.allow_withdraw).toBeDefined();

            // SPL token policy
            expect(config.validation_config.fee_payer_policy.spl_token).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_transfer).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_burn).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_close_account).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_approve).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_revoke).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_set_authority).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_mint_to).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_freeze_account).toBeDefined();
            expect(config.validation_config.fee_payer_policy.spl_token.allow_thaw_account).toBeDefined();

            // Token2022 policy
            expect(config.validation_config.fee_payer_policy.token_2022).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_transfer).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_burn).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_close_account).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_approve).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_revoke).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_set_authority).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_mint_to).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_freeze_account).toBeDefined();
            expect(config.validation_config.fee_payer_policy.token_2022.allow_thaw_account).toBeDefined();
            expect(config.enabled_methods).toBeDefined();
            expect(config.enabled_methods.liveness).toBeDefined();
            expect(config.enabled_methods.estimate_transaction_fee).toBeDefined();
            expect(config.enabled_methods.get_supported_tokens).toBeDefined();
            expect(config.enabled_methods.sign_transaction).toBeDefined();
            expect(config.enabled_methods.sign_and_send_transaction).toBeDefined();
            expect(config.enabled_methods.transfer_transaction).toBeDefined();
            expect(config.enabled_methods.get_blockhash).toBeDefined();
            expect(config.enabled_methods.get_config).toBeDefined();
            expect(config.enabled_methods.get_version).toBeDefined();
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

        it('should get version', async () => {
            const { version } = await client.getVersion();
            expect(version).toBeDefined();
            expect(typeof version).toBe('string');
            expect(version.length).toBeGreaterThan(0);
            // Version should follow semver format (e.g., "2.1.0" or "2.1.0-beta.0")
            expect(version).toMatch(/^\d+\.\d+\.\d+/);
        });
    });

    describe('Transaction Operations', () => {
        it('should create transfer transaction (DEPRECATED endpoint)', async () => {
            const request = {
                amount: 1000000, // 1 USDC
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress, // user specifies destination
            };

            const response = await client.transferTransaction(request);
            expect(response).toBeDefined();
            expect(response.transaction).toBeDefined();
            expect(response.blockhash).toBeDefined();
            expect(response.message).toBeDefined();
            expect(response.signer_pubkey).toBeDefined();
        });

        it('should estimate transaction fee', async () => {
            // First create a transaction
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
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
                destination: koraAddress,
            };

            const { transaction } = await client.transferTransaction(transferRequest);

            const signResult = await client.signTransaction({
                transaction,
            });

            expect(signResult).toBeDefined();
            expect(signResult.signed_transaction).toBeDefined();
        });

        it('should sign and send transaction', async () => {
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
            };

            const { transaction: transactionString } = await client.transferTransaction(transferRequest);

            const transaction = transactionFromBase64(transactionString);
            // Partially sign transaction with test wallet before sending
            // Kora will add fee payer signature via signAndSendTransaction
            const signedTransaction = await partiallySignTransaction([testWallet.keyPair], transaction);
            const base64SignedTransaction = transactionToBase64(signedTransaction);
            const signResult = await client.signAndSendTransaction({
                transaction: base64SignedTransaction,
            });

            expect(signResult).toBeDefined();
            expect(signResult.signed_transaction).toBeDefined();
        });

        it('should get payment instruction', async () => {
            const transferRequest = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
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

    describe('Bundle Operations', () => {
        it('should sign bundle of transactions', async () => {
            // Create two transfer transactions for the bundle
            const transferRequest1 = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
            };
            const transferRequest2 = {
                amount: 500000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
            };

            const { transaction: tx1String } = await client.transferTransaction(transferRequest1);
            const { transaction: tx2String } = await client.transferTransaction(transferRequest2);

            // Partially sign both transactions with test wallet
            const tx1 = transactionFromBase64(tx1String);
            const tx2 = transactionFromBase64(tx2String);
            const signedTx1 = await partiallySignTransaction([testWallet.keyPair], tx1);
            const signedTx2 = await partiallySignTransaction([testWallet.keyPair], tx2);
            const base64Tx1 = transactionToBase64(signedTx1);
            const base64Tx2 = transactionToBase64(signedTx2);

            const result = await client.signBundle({
                transactions: [base64Tx1, base64Tx2],
            });

            expect(result).toBeDefined();
            expect(result.signed_transactions).toBeDefined();
            expect(Array.isArray(result.signed_transactions)).toBe(true);
            expect(result.signed_transactions.length).toBe(2);
            expect(result.signer_pubkey).toBeDefined();
        });

        it('should sign and send bundle of transactions', async () => {
            // Create two transfer transactions for the bundle
            const transferRequest1 = {
                amount: 1000000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
            };
            const transferRequest2 = {
                amount: 500000,
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
            };

            const { transaction: tx1String } = await client.transferTransaction(transferRequest1);
            const { transaction: tx2String } = await client.transferTransaction(transferRequest2);

            // Partially sign both transactions with test wallet
            const tx1 = transactionFromBase64(tx1String);
            const tx2 = transactionFromBase64(tx2String);
            const signedTx1 = await partiallySignTransaction([testWallet.keyPair], tx1);
            const signedTx2 = await partiallySignTransaction([testWallet.keyPair], tx2);
            const base64Tx1 = transactionToBase64(signedTx1);
            const base64Tx2 = transactionToBase64(signedTx2);

            const result = await client.signAndSendBundle({
                transactions: [base64Tx1, base64Tx2],
            });

            expect(result).toBeDefined();
            expect(result.signed_transactions).toBeDefined();
            expect(Array.isArray(result.signed_transactions)).toBe(true);
            expect(result.signed_transactions.length).toBe(2);
            expect(result.signer_pubkey).toBeDefined();
            expect(result.bundle_uuid).toBeDefined();
            expect(typeof result.bundle_uuid).toBe('string');
        });
    });

    describe('Error Handling', () => {
        it('should handle invalid token address', async () => {
            const request = {
                amount: 1000000,
                token: 'InvalidTokenAddress',
                source: testWalletAddress,
                destination: koraAddress,
            };

            await expect(client.transferTransaction(request)).rejects.toThrow();
        });

        it('should handle invalid amount', async () => {
            const request = {
                amount: -1, // Invalid amount
                token: usdcMint,
                source: testWalletAddress,
                destination: koraAddress,
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
                destination: koraAddress,
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
                destination: koraAddress,
            };

            // Create and sign the transaction
            const { transaction } = await client.transferTransaction(request);
            const signResult = await client.signTransaction({ transaction });

            expect(signResult.signed_transaction).toBeDefined();
        });

        it('should reject transaction with non-allowed token', async () => {
            const invalidTokenMint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'; // Mainnet USDC mint
            const request = {
                amount: 1000000,
                token: invalidTokenMint,
                source: testWalletAddress,
                destination: koraAddress,
            };

            await expect(client.transferTransaction(request)).rejects.toThrow();
        });
    });
});
