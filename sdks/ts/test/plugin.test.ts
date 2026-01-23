import type { Address, Base64EncodedWireTransaction, Blockhash, Signature } from '@solana/kit';
import { createEmptyClient } from '@solana/kit';

import { type KoraApi, koraPlugin } from '../src/plugin.js';
import type {
    GetVersionResponse,
    KitBlockhashResponse,
    KitConfigResponse,
    KitEstimateBundleFeeResponse,
    KitEstimateFeeResponse,
    KitPayerSignerResponse,
    KitPaymentInstructionResponse,
    KitSignAndSendBundleResponse,
    KitSignAndSendTransactionResponse,
    KitSignBundleResponse,
    KitSignTransactionResponse,
    KitSupportedTokensResponse,
    KoraPluginConfig,
} from '../src/types/index.js';

// Mock fetch globally
const mockFetch = jest.fn();
global.fetch = mockFetch;

describe('Kora Kit Plugin', () => {
    const mockEndpoint = 'http://localhost:8080';
    const mockConfig: KoraPluginConfig = {
        endpoint: mockEndpoint,
    };

    // Helper to mock successful RPC response
    const mockSuccessfulResponse = (result: unknown) => {
        mockFetch.mockResolvedValueOnce({
            json: jest.fn().mockResolvedValueOnce({
                id: 1,
                jsonrpc: '2.0',
                result,
            }),
        });
    };

    // Helper to mock error response
    const mockErrorResponse = (error: { code: number; message: string }) => {
        mockFetch.mockResolvedValueOnce({
            json: jest.fn().mockResolvedValueOnce({
                error,
                id: 1,
                jsonrpc: '2.0',
            }),
        });
    };

    beforeEach(() => {
        mockFetch.mockClear();
    });

    describe('Plugin Composition', () => {
        it('should add kora property to client', () => {
            const baseClient = { existing: 'property' };
            const plugin = koraPlugin(mockConfig);
            const enhanced = plugin(baseClient);

            expect(enhanced.existing).toBe('property');
            expect(enhanced.kora).toBeDefined();
            expect(typeof enhanced.kora.getConfig).toBe('function');
            expect(typeof enhanced.kora.getPayerSigner).toBe('function');
            expect(typeof enhanced.kora.getBlockhash).toBe('function');
            expect(typeof enhanced.kora.getVersion).toBe('function');
            expect(typeof enhanced.kora.getSupportedTokens).toBe('function');
            expect(typeof enhanced.kora.estimateTransactionFee).toBe('function');
            expect(typeof enhanced.kora.estimateBundleFee).toBe('function');
            expect(typeof enhanced.kora.signTransaction).toBe('function');
            expect(typeof enhanced.kora.signAndSendTransaction).toBe('function');
            expect(typeof enhanced.kora.signBundle).toBe('function');
            expect(typeof enhanced.kora.signAndSendBundle).toBe('function');
            expect(typeof enhanced.kora.getPaymentInstruction).toBe('function');
        });

        it('should work with empty client object', () => {
            const plugin = koraPlugin(mockConfig);
            const enhanced = plugin({});

            expect(enhanced.kora).toBeDefined();
        });

        it('should support authentication options', () => {
            const authConfig: KoraPluginConfig = {
                apiKey: 'test-api-key',
                endpoint: mockEndpoint,
                hmacSecret: 'test-hmac-secret',
            };

            const plugin = koraPlugin(authConfig);
            const enhanced = plugin({});

            expect(enhanced.kora).toBeDefined();
        });
    });

    describe('Type Casting', () => {
        let kora: KoraApi;

        beforeEach(() => {
            const plugin = koraPlugin(mockConfig);
            const client = plugin({});
            kora = client.kora;
        });

        describe('getConfig', () => {
            it('should return Kit-typed Address arrays', async () => {
                const rawResponse = {
                    enabled_methods: {
                        estimate_transaction_fee: true,
                        get_blockhash: true,
                        get_config: true,
                        get_supported_tokens: true,
                        liveness: true,
                        sign_and_send_transaction: true,
                        sign_transaction: true,
                        transfer_transaction: true,
                    },
                    fee_payers: ['DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7'],
                    validation_config: {
                        allowed_programs: ['11111111111111111111111111111111'],
                        allowed_spl_paid_tokens: ['EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
                        allowed_tokens: ['EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
                        disallowed_accounts: [],
                        fee_payer_policy: {
                            spl_token: {
                                allow_approve: true,
                                allow_burn: true,
                                allow_close_account: true,
                                allow_freeze_account: true,
                                allow_mint_to: true,
                                allow_revoke: true,
                                allow_set_authority: true,
                                allow_thaw_account: true,
                                allow_transfer: true,
                            },
                            system: {
                                allow_allocate: true,
                                allow_assign: true,
                                allow_create_account: true,
                                allow_transfer: true,
                                nonce: {
                                    allow_advance: true,
                                    allow_authorize: true,
                                    allow_initialize: true,
                                    allow_withdraw: true,
                                },
                            },
                            token_2022: {
                                allow_approve: true,
                                allow_burn: true,
                                allow_close_account: true,
                                allow_freeze_account: true,
                                allow_mint_to: true,
                                allow_revoke: true,
                                allow_set_authority: true,
                                allow_thaw_account: true,
                                allow_transfer: true,
                            },
                        },
                        max_allowed_lamports: 1000000,
                        max_signatures: 10,
                        price: { margin: 0.1, type: 'margin' },
                        price_source: 'Jupiter',
                        token2022: {
                            blocked_account_extensions: [],
                            blocked_mint_extensions: [],
                        },
                    },
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitConfigResponse = await kora.getConfig();

                // Verify type casting - these should be Address types
                expect(result.fee_payers).toHaveLength(1);
                expect(result.fee_payers[0]).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');

                expect(result.validation_config.allowed_programs).toHaveLength(1);
                expect(result.validation_config.allowed_programs[0]).toBe('11111111111111111111111111111111');

                expect(result.validation_config.allowed_tokens).toHaveLength(1);
                expect(result.validation_config.allowed_tokens[0]).toBe('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
            });
        });

        describe('getPayerSigner', () => {
            it('should return Kit-typed Address fields', async () => {
                const rawResponse = {
                    payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                    signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitPayerSignerResponse = await kora.getPayerSigner();

                // Type assertion - these should be Address types
                const signerAddr: Address = result.signer_address;
                const paymentAddr: Address = result.payment_address;

                expect(signerAddr).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(paymentAddr).toBe('PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('getBlockhash', () => {
            it('should return Kit-typed Blockhash field', async () => {
                const rawResponse = {
                    blockhash: '4NxM2D4kQcipkzMWBWQME5YSVnj5kT8QKA7rvb3rKLvE',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitBlockhashResponse = await kora.getBlockhash();

                // Type assertion - should be Blockhash type
                const hash: Blockhash = result.blockhash;
                expect(hash).toBe('4NxM2D4kQcipkzMWBWQME5YSVnj5kT8QKA7rvb3rKLvE');
            });
        });

        describe('getVersion', () => {
            it('should return version string', async () => {
                const rawResponse = {
                    version: '2.0.0',
                };

                mockSuccessfulResponse(rawResponse);

                const result: GetVersionResponse = await kora.getVersion();

                expect(result.version).toBe('2.0.0');
            });
        });

        describe('getSupportedTokens', () => {
            it('should return Kit-typed Address array', async () => {
                const rawResponse = {
                    tokens: [
                        'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
                        'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',
                    ],
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitSupportedTokensResponse = await kora.getSupportedTokens();

                // Type assertion - these should be Address types
                expect(result.tokens).toHaveLength(2);
                const token0: Address = result.tokens[0];
                const token1: Address = result.tokens[1];

                expect(token0).toBe('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
                expect(token1).toBe('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB');
            });
        });

        describe('estimateTransactionFee', () => {
            it('should return Kit-typed Address fields', async () => {
                const rawResponse = {
                    fee_in_lamports: 5000,
                    fee_in_token: 50,
                    payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitEstimateFeeResponse = await kora.estimateTransactionFee({
                    fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
                    transaction: 'base64EncodedTransaction',
                });

                // Type assertions
                const signerPubkey: Address = result.signer_pubkey;
                const paymentAddr: Address = result.payment_address;

                expect(result.fee_in_lamports).toBe(5000);
                expect(result.fee_in_token).toBe(50);
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(paymentAddr).toBe('PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('estimateBundleFee', () => {
            it('should return Kit-typed Address fields for bundle', async () => {
                const rawResponse = {
                    fee_in_lamports: 15000,
                    fee_in_token: 150,
                    payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitEstimateBundleFeeResponse = await kora.estimateBundleFee({
                    fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
                    transactions: ['base64Tx1', 'base64Tx2', 'base64Tx3'],
                });

                // Type assertions
                const signerPubkey: Address = result.signer_pubkey;
                const paymentAddr: Address = result.payment_address;

                expect(result.fee_in_lamports).toBe(15000);
                expect(result.fee_in_token).toBe(150);
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(paymentAddr).toBe('PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('signTransaction', () => {
            it('should return Kit-typed response with Base64EncodedWireTransaction', async () => {
                const rawResponse = {
                    signed_transaction: 'base64SignedTransaction',
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitSignTransactionResponse = await kora.signTransaction({
                    transaction: 'base64EncodedTransaction',
                });

                // Type assertions - verify Kit types
                const signedTx: Base64EncodedWireTransaction = result.signed_transaction;
                const signerPubkey: Address = result.signer_pubkey;

                expect(signedTx).toBe('base64SignedTransaction');
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('signAndSendTransaction', () => {
            it('should return Kit-typed response with Signature and Base64EncodedWireTransaction', async () => {
                // Use a valid base58 signature (88 characters, valid base58 alphabet)
                const mockSignature =
                    '5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW';
                const rawResponse = {
                    signature: mockSignature,
                    signed_transaction: 'base64SignedTransaction',
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitSignAndSendTransactionResponse = await kora.signAndSendTransaction({
                    transaction: 'base64EncodedTransaction',
                });

                // Type assertions - verify Kit types
                const sig: Signature = result.signature;
                const signedTx: Base64EncodedWireTransaction = result.signed_transaction;
                const signerPubkey: Address = result.signer_pubkey;

                expect(sig).toBe(mockSignature);
                expect(signedTx).toBe('base64SignedTransaction');
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('signBundle', () => {
            it('should return Kit-typed response with Base64EncodedWireTransaction array', async () => {
                const rawResponse = {
                    signed_transactions: ['base64SignedTx1', 'base64SignedTx2'],
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitSignBundleResponse = await kora.signBundle({
                    transactions: ['base64Tx1', 'base64Tx2'],
                });

                // Type assertions - verify Kit types
                const signedTxs: Base64EncodedWireTransaction[] = result.signed_transactions;
                const signerPubkey: Address = result.signer_pubkey;

                expect(signedTxs).toHaveLength(2);
                expect(signedTxs[0]).toBe('base64SignedTx1');
                expect(signedTxs[1]).toBe('base64SignedTx2');
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            });
        });

        describe('signAndSendBundle', () => {
            it('should return Kit-typed response with Base64EncodedWireTransaction array and bundle UUID', async () => {
                const rawResponse = {
                    bundle_uuid: 'jito-bundle-uuid-12345',
                    signed_transactions: ['base64SignedTx1', 'base64SignedTx2'],
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                mockSuccessfulResponse(rawResponse);

                const result: KitSignAndSendBundleResponse = await kora.signAndSendBundle({
                    transactions: ['base64Tx1', 'base64Tx2'],
                });

                // Type assertions - verify Kit types
                const signedTxs: Base64EncodedWireTransaction[] = result.signed_transactions;
                const signerPubkey: Address = result.signer_pubkey;

                expect(signedTxs).toHaveLength(2);
                expect(signedTxs[0]).toBe('base64SignedTx1');
                expect(signedTxs[1]).toBe('base64SignedTx2');
                expect(signerPubkey).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(result.bundle_uuid).toBe('jito-bundle-uuid-12345');
            });
        });

        describe('getPaymentInstruction', () => {
            it('should return Kit-typed response with Base64EncodedWireTransaction and Address fields', async () => {
                const mockFeeEstimate = {
                    fee_in_lamports: 5000,
                    fee_in_token: 50000,
                    payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                    signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                };

                const testTx =
                    'Aoq7ymA5OGP+gmDXiY5m3cYXlY2Rz/a/gFjOgt9ZuoCS7UzuiGGaEnW2OOtvHvMQHkkD7Z4LRF5B63ftu+1oZwIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgECB1urjQEjgFgzqYhJ8IXJeSg4cJP1j1g2CJstOQTDchOKUzqH3PxgGW3c4V3vZV05A5Y30/MggOBs0Kd00s1JEwg5TaEeaV4+KL2y7fXIAuf6cN0ZQitbhY+G9ExtBSChspOXPgNcy9pYpETe4bmB+fg4bfZx1tnicA/kIyyubczAmbcIKIuniNOOQYG2ggKCz8NjEsHVezrWMatndu1wk6J5miGP26J6Vwp31AljiAajAFuP0D9mWJwSeFuA7J5rPwbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpd/O36SW02zRtNtqk6GFeip2+yBQsVTeSbLL4rWJRkd4CBgQCBQQBCgxAQg8AAAAAAAYGBAIFAwEKDBAnAAAAAAAABg==';

                mockSuccessfulResponse(mockFeeEstimate);

                const result: KitPaymentInstructionResponse = await kora.getPaymentInstruction({
                    fee_token: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
                    source_wallet: '11111111111111111111111111111111',
                    token_program_id: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                    transaction: testTx,
                });

                // Type assertions - verify Kit types
                const originalTx: Base64EncodedWireTransaction = result.original_transaction;
                const paymentToken: Address = result.payment_token;
                const paymentAddr: Address = result.payment_address;
                const signerAddr: Address = result.signer_address;

                expect(originalTx).toBe(testTx);
                expect(paymentToken).toBe('4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU');
                expect(paymentAddr).toBe('PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(signerAddr).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
                expect(result.payment_amount).toBe(50000);
            });
        });
    });

    describe('Error Handling', () => {
        let kora: KoraApi;

        beforeEach(() => {
            const plugin = koraPlugin(mockConfig);
            const client = plugin({});
            kora = client.kora;
        });

        it('should propagate RPC errors', async () => {
            mockErrorResponse({ code: -32601, message: 'Method not found' });

            await expect(kora.getConfig()).rejects.toThrow('RPC Error -32601: Method not found');
        });

        it('should propagate network errors', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Network error'));

            await expect(kora.getConfig()).rejects.toThrow('Network error');
        });
    });

    describe('KoraApi Type Export', () => {
        it('should export KoraApi type correctly', () => {
            // This test verifies the KoraApi type is correctly exported
            const plugin = koraPlugin(mockConfig);
            const client = plugin({});

            // Type check - assign to KoraApi type
            const api: KoraApi = client.kora;
            expect(api).toBeDefined();
        });
    });

    describe('createEmptyClient Integration', () => {
        it('should initialize kora property on Kit client', () => {
            const client = createEmptyClient().use(koraPlugin(mockConfig));

            expect(client).toHaveProperty('kora');
            expect(client.kora).toBeDefined();
        });

        it('should expose all Kora RPC methods', () => {
            const client = createEmptyClient().use(koraPlugin(mockConfig));

            expect(typeof client.kora.getConfig).toBe('function');
            expect(typeof client.kora.getPayerSigner).toBe('function');
            expect(typeof client.kora.getBlockhash).toBe('function');
            expect(typeof client.kora.getVersion).toBe('function');
            expect(typeof client.kora.getSupportedTokens).toBe('function');
            expect(typeof client.kora.estimateTransactionFee).toBe('function');
            expect(typeof client.kora.estimateBundleFee).toBe('function');
            expect(typeof client.kora.signTransaction).toBe('function');
            expect(typeof client.kora.signAndSendTransaction).toBe('function');
            expect(typeof client.kora.signBundle).toBe('function');
            expect(typeof client.kora.signAndSendBundle).toBe('function');
            expect(typeof client.kora.getPaymentInstruction).toBe('function');
        });

        it('should work with authentication config', () => {
            const authConfig: KoraPluginConfig = {
                apiKey: 'test-api-key',
                endpoint: mockEndpoint,
                hmacSecret: 'test-hmac-secret',
            };

            const client = createEmptyClient().use(koraPlugin(authConfig));

            expect(client.kora).toBeDefined();
            expect(typeof client.kora.getConfig).toBe('function');
        });

        it('should compose with other plugins', () => {
            // Simulate another plugin that adds a different property
            const otherPlugin = <T extends object>(c: T) => ({
                ...c,
                other: { foo: () => 'bar' },
            });

            const client = createEmptyClient().use(koraPlugin(mockConfig)).use(otherPlugin);

            // Both plugins should be available
            expect(client.kora).toBeDefined();
            expect(client.other).toBeDefined();
            expect(typeof client.kora.getConfig).toBe('function');
            expect(client.other.foo()).toBe('bar');
        });

        it('should call RPC methods correctly', async () => {
            const mockResponse = {
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            mockSuccessfulResponse(mockResponse);

            const client = createEmptyClient().use(koraPlugin(mockConfig));
            const result = await client.kora.getPayerSigner();

            expect(result.signer_address).toBe('DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
            expect(result.payment_address).toBe('PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7');
        });
    });
});
