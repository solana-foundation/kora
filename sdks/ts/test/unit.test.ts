import { TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';

import { KoraClient } from '../src/client.js';
import {
    Config,
    EstimateTransactionFeeRequest,
    EstimateTransactionFeeResponse,
    GetBlockhashResponse,
    GetPayerSignerResponse,
    GetSupportedTokensResponse,
    GetVersionResponse,
    SignAndSendBundleRequest,
    SignAndSendBundleResponse,
    SignAndSendTransactionRequest,
    SignAndSendTransactionResponse,
    SignBundleRequest,
    SignBundleResponse,
    SignTransactionRequest,
    SignTransactionResponse,
} from '../src/types/index.js';
import { getInstructionsFromBase64Message } from '../src/utils/transaction.js';

// Mock fetch globally
const mockFetch = jest.fn();
global.fetch = mockFetch;

describe('KoraClient Unit Tests', () => {
    let client: KoraClient;
    const mockRpcUrl = 'http://localhost:8080';

    // Helper Functions
    const mockSuccessfulResponse = (result: any) => {
        mockFetch.mockResolvedValueOnce({
            json: jest.fn().mockResolvedValueOnce({
                id: 1,
                jsonrpc: '2.0',
                result,
            }),
        });
    };

    const mockErrorResponse = (error: any) => {
        mockFetch.mockResolvedValueOnce({
            json: jest.fn().mockResolvedValueOnce({
                error,
                id: 1,
                jsonrpc: '2.0',
            }),
        });
    };

    const expectRpcCall = (method: string, params: any = undefined) => {
        expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
            body: JSON.stringify({
                id: 1,
                jsonrpc: '2.0',
                method,
                params,
            }),
            headers: {
                'Content-Type': 'application/json',
            },
            method: 'POST',
        });
    };

    const testSuccessfulRpcMethod = async (
        methodName: string,
        clientMethod: () => Promise<any>,
        expectedResult: any,
        params: any = undefined,
    ) => {
        mockSuccessfulResponse(expectedResult);
        const result = await clientMethod();
        expect(result).toEqual(expectedResult);
        expectRpcCall(methodName, params);
    };

    beforeEach(() => {
        client = new KoraClient({ rpcUrl: mockRpcUrl });
        mockFetch.mockClear();
    });

    afterEach(() => {
        jest.resetAllMocks();
    });

    describe('Constructor', () => {
        it('should create KoraClient instance with provided RPC URL', () => {
            const testUrl = 'https://api.example.com';
            const testClient = new KoraClient({ rpcUrl: testUrl });
            expect(testClient).toBeInstanceOf(KoraClient);
        });
    });

    describe('RPC Request Handling', () => {
        it('should handle successful RPC responses', async () => {
            const mockResult = { value: 'test' };
            await testSuccessfulRpcMethod('getConfig', () => client.getConfig(), mockResult);
        });

        it('should handle RPC error responses', async () => {
            const mockError = { code: -32601, message: 'Method not found' };
            mockErrorResponse(mockError);
            await expect(client.getConfig()).rejects.toThrow('RPC Error -32601: Method not found');
        });

        it('should handle network errors', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Network error'));
            await expect(client.getConfig()).rejects.toThrow('Network error');
        });
    });

    describe('getConfig', () => {
        it('should return configuration', async () => {
            const mockConfig: Config = {
                enabled_methods: {
                    estimate_bundle_fee: true,
                    estimate_transaction_fee: true,
                    get_blockhash: true,
                    get_config: true,
                    get_payer_signer: true,
                    get_supported_tokens: true,
                    get_version: true,
                    liveness: true,
                    sign_and_send_bundle: true,
                    sign_and_send_transaction: true,
                    sign_bundle: true,
                    sign_transaction: true,
                    transfer_transaction: true,
                },
                fee_payers: ['test_fee_payer_address'],
                validation_config: {
                    allowed_programs: ['program1', 'program2'],
                    allowed_spl_paid_tokens: ['spl_token1'],
                    allowed_tokens: ['token1', 'token2'],
                    disallowed_accounts: ['account1'],
                    fee_payer_policy: {
                        spl_token: {
                            allow_approve: true,
                            allow_burn: true,
                            allow_close_account: true,
                            allow_freeze_account: true,
                            allow_initialize_account: true,
                            allow_initialize_mint: true,
                            allow_initialize_multisig: true,
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
                            allow_initialize_account: true,
                            allow_initialize_mint: true,
                            allow_initialize_multisig: true,
                            allow_mint_to: true,
                            allow_revoke: true,
                            allow_set_authority: true,
                            allow_thaw_account: true,
                            allow_transfer: false,
                        },
                    },
                    max_allowed_lamports: 1000000,
                    max_signatures: 10,
                    price: {
                        margin: 0.1,
                        type: 'margin',
                    },
                    price_source: 'Jupiter',
                    token2022: {
                        blocked_account_extensions: ['account_extension1', 'account_extension2'],
                        blocked_mint_extensions: ['extension1', 'extension2'],
                    },
                },
            };

            await testSuccessfulRpcMethod('getConfig', () => client.getConfig(), mockConfig);
        });
    });

    describe('getBlockhash', () => {
        it('should return blockhash', async () => {
            const mockResponse: GetBlockhashResponse = {
                blockhash: 'test_blockhash_value',
            };

            await testSuccessfulRpcMethod('getBlockhash', () => client.getBlockhash(), mockResponse);
        });
    });

    describe('getVersion', () => {
        it('should return server version', async () => {
            const mockResponse: GetVersionResponse = {
                version: '2.1.0-beta.0',
            };

            await testSuccessfulRpcMethod('getVersion', () => client.getVersion(), mockResponse);
        });
    });

    describe('getSupportedTokens', () => {
        it('should return supported tokens list', async () => {
            const mockResponse: GetSupportedTokensResponse = {
                tokens: ['SOL', 'USDC', 'USDT'],
            };

            await testSuccessfulRpcMethod('getSupportedTokens', () => client.getSupportedTokens(), mockResponse);
        });
    });

    describe('getPayerSigner', () => {
        it('should return payer signer and payment destination', async () => {
            const mockResponse: GetPayerSignerResponse = {
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            await testSuccessfulRpcMethod('getPayerSigner', () => client.getPayerSigner(), mockResponse);
        });

        it('should return same address for signer and payment_destination when no separate paymaster', async () => {
            const mockResponse: GetPayerSignerResponse = {
                payment_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            await testSuccessfulRpcMethod('getPayerSigner', () => client.getPayerSigner(), mockResponse);
            expect(mockResponse.signer_address).toBe(mockResponse.payment_address);
        });
    });

    describe('estimateTransactionFee', () => {
        it('should estimate transaction fee', async () => {
            const request: EstimateTransactionFeeRequest = {
                fee_token: 'SOL',
                transaction: 'base64_encoded_transaction',
            };
            const mockResponse: EstimateTransactionFeeResponse = {
                fee_in_lamports: 5000,
                fee_in_token: 25,
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            await testSuccessfulRpcMethod(
                'estimateTransactionFee',
                () => client.estimateTransactionFee(request),
                mockResponse,
                request,
            );
        });
    });

    describe('signTransaction', () => {
        it('should sign transaction', async () => {
            const request: SignTransactionRequest = {
                transaction: 'base64_encoded_transaction',
            };
            const mockResponse: SignTransactionResponse = {
                signed_transaction: 'base64_signed_transaction',
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod(
                'signTransaction',
                () => client.signTransaction(request),
                mockResponse,
                request,
            );
        });
    });

    describe('signAndSendTransaction', () => {
        it('should sign and send transaction', async () => {
            const request: SignAndSendTransactionRequest = {
                transaction: 'base64_encoded_transaction',
            };
            const mockResponse: SignAndSendTransactionResponse = {
                signature: 'transaction_signature',
                signed_transaction: 'base64_signed_transaction',
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod(
                'signAndSendTransaction',
                () => client.signAndSendTransaction(request),
                mockResponse,
                request,
            );
        });
    });

    describe('signBundle', () => {
        it('should sign bundle of transactions', async () => {
            const request: SignBundleRequest = {
                transactions: ['base64_tx_1', 'base64_tx_2'],
            };
            const mockResponse: SignBundleResponse = {
                signed_transactions: ['base64_signed_tx_1', 'base64_signed_tx_2'],
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod('signBundle', () => client.signBundle(request), mockResponse, request);
        });

        it('should handle RPC error', async () => {
            const request: SignBundleRequest = {
                transactions: ['base64_tx_1'],
            };
            const mockError = { code: -32000, message: 'Bundle validation failed' };
            mockErrorResponse(mockError);
            await expect(client.signBundle(request)).rejects.toThrow('RPC Error -32000: Bundle validation failed');
        });
    });

    describe('signAndSendBundle', () => {
        it('should sign and send bundle of transactions', async () => {
            const request: SignAndSendBundleRequest = {
                transactions: ['base64_tx_1', 'base64_tx_2'],
            };
            const mockResponse: SignAndSendBundleResponse = {
                bundle_uuid: 'test-bundle-uuid-123',
                signed_transactions: ['base64_signed_tx_1', 'base64_signed_tx_2'],
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod(
                'signAndSendBundle',
                () => client.signAndSendBundle(request),
                mockResponse,
                request,
            );
        });

        it('should handle RPC error', async () => {
            const request: SignAndSendBundleRequest = {
                transactions: ['base64_tx_1'],
            };
            const mockError = { code: -32000, message: 'Jito submission failed' };
            mockErrorResponse(mockError);
            await expect(client.signAndSendBundle(request)).rejects.toThrow('RPC Error -32000: Jito submission failed');
        });
    });

    describe('getPaymentInstruction', () => {
        const _mockConfig: Config = {
            enabled_methods: {
                estimate_bundle_fee: true,
                estimate_transaction_fee: true,
                get_blockhash: true,
                get_config: true,
                get_payer_signer: true,
                get_supported_tokens: true,
                get_version: true,
                liveness: true,
                sign_and_send_bundle: true,
                sign_and_send_transaction: true,
                sign_bundle: true,
                sign_transaction: true,
                transfer_transaction: true,
            },
            fee_payers: ['11111111111111111111111111111111'],
            validation_config: {
                allowed_programs: ['program1'],
                allowed_spl_paid_tokens: ['4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU'],
                allowed_tokens: ['token1'],
                disallowed_accounts: [],
                fee_payer_policy: {
                    spl_token: {
                        allow_approve: true,
                        allow_burn: true,
                        allow_close_account: true,
                        allow_freeze_account: true,
                        allow_initialize_account: true,
                        allow_initialize_mint: true,
                        allow_initialize_multisig: true,
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
                        allow_initialize_account: true,
                        allow_initialize_mint: true,
                        allow_initialize_multisig: true,
                        allow_mint_to: true,
                        allow_revoke: true,
                        allow_set_authority: true,
                        allow_thaw_account: true,
                        allow_transfer: true,
                    },
                },
                max_allowed_lamports: 1000000,
                max_signatures: 10,
                price: {
                    margin: 0.1,
                    type: 'margin',
                },
                price_source: 'Jupiter',
                token2022: {
                    blocked_account_extensions: [],
                    blocked_mint_extensions: [],
                },
            },
        };

        const mockFeeEstimate: EstimateTransactionFeeResponse = {
            fee_in_lamports: 5000,
            fee_in_token: 50000,
            payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            signer_pubkey: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
        };

        // Create a mock base64-encoded transaction
        // This is a minimal valid transaction structure
        const mockTransactionBase64 =
            'Aoq7ymA5OGP+gmDXiY5m3cYXlY2Rz/a/gFjOgt9ZuoCS7UzuiGGaEnW2OOtvHvMQHkkD7Z4LRF5B63ftu+1oZwIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgECB1urjQEjgFgzqYhJ8IXJeSg4cJP1j1g2CJstOQTDchOKUzqH3PxgGW3c4V3vZV05A5Y30/MggOBs0Kd00s1JEwg5TaEeaV4+KL2y7fXIAuf6cN0ZQitbhY+G9ExtBSChspOXPgNcy9pYpETe4bmB+fg4bfZx1tnicA/kIyyubczAmbcIKIuniNOOQYG2ggKCz8NjEsHVezrWMatndu1wk6J5miGP26J6Vwp31AljiAajAFuP0D9mWJwSeFuA7J5rPwbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpd/O36SW02zRtNtqk6GFeip2+yBQsVTeSbLL4rWJRkd4CBgQCBQQBCgxAQg8AAAAAAAYGBAIFAwEKDBAnAAAAAAAABg==';

        const validRequest = {
            fee_token: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
            source_wallet: '11111111111111111111111111111111',
            token_program_id: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
            transaction: mockTransactionBase64,
        };

        beforeEach(() => {
            // Mock console.log to avoid noise in tests
            jest.spyOn(console, 'log').mockImplementation();
        });

        afterEach(() => {
            jest.restoreAllMocks();
        });

        it('should successfully append payment instruction', async () => {
            // Mock estimateTransactionFee call
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    id: 1,
                    jsonrpc: '2.0',
                    result: mockFeeEstimate,
                }),
            });

            const result = await client.getPaymentInstruction(validRequest);

            expect(result).toEqual({
                original_transaction: validRequest.transaction,
                payment_address: mockFeeEstimate.payment_address,
                payment_amount: mockFeeEstimate.fee_in_token,
                payment_instruction: expect.objectContaining({
                    accounts: [
                        expect.objectContaining({
                            role: 1, // writable
                        }), // Source token account
                        expect.objectContaining({
                            role: 1, // writable
                        }), // Destination token account
                        expect.objectContaining({
                            // readonly-signer
                            address: validRequest.source_wallet,
                            role: 2,
                            signer: expect.objectContaining({
                                address: validRequest.source_wallet,
                            }),
                        }), // Authority
                    ],
                    data: expect.any(Uint8Array),
                    programAddress: TOKEN_PROGRAM_ADDRESS,
                }),
                payment_token: validRequest.fee_token,
                signer_address: mockFeeEstimate.signer_pubkey,
            });

            // Verify only estimateTransactionFee was called
            expect(mockFetch).toHaveBeenCalledTimes(1);
            expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
                body: JSON.stringify({
                    id: 1,
                    jsonrpc: '2.0',
                    method: 'estimateTransactionFee',
                    params: {
                        fee_token: validRequest.fee_token,
                        transaction: validRequest.transaction,
                    },
                }),
                headers: {
                    'Content-Type': 'application/json',
                },
                method: 'POST',
            });
        });

        it('should handle fixed pricing configuration', async () => {
            // Mock estimateTransactionFee call
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    id: 1,
                    jsonrpc: '2.0',
                    result: mockFeeEstimate,
                }),
            });

            const result = await client.getPaymentInstruction(validRequest);

            expect(result.payment_amount).toBe(mockFeeEstimate.fee_in_token);
            expect(result.payment_token).toBe(validRequest.fee_token);
        });

        it('should throw error for invalid addresses', async () => {
            const invalidRequests = [
                { ...validRequest, source_wallet: 'invalid_address' },
                { ...validRequest, fee_token: 'invalid_token' },
                { ...validRequest, token_program_id: 'invalid_program' },
            ];

            for (const invalidRequest of invalidRequests) {
                await expect(client.getPaymentInstruction(invalidRequest)).rejects.toThrow();
            }
        });

        it('should handle estimateTransactionFee RPC error', async () => {
            // Mock failed estimateTransactionFee
            const mockError = { code: -32602, message: 'Invalid transaction' };
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    error: mockError,
                    id: 1,
                    jsonrpc: '2.0',
                }),
            });

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow(
                'RPC Error -32602: Invalid transaction',
            );
        });

        it('should handle network errors', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Network error'));

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow('Network error');
        });

        it('should return correct payment details in response', async () => {
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    id: 1,
                    jsonrpc: '2.0',
                    result: mockFeeEstimate,
                }),
            });

            const result = await client.getPaymentInstruction(validRequest);

            expect(result).toMatchObject({
                original_transaction: validRequest.transaction,
                payment_address: mockFeeEstimate.payment_address,
                payment_amount: mockFeeEstimate.fee_in_token,
                payment_instruction: expect.any(Object),
                payment_token: validRequest.fee_token,
                signer_address: mockFeeEstimate.signer_pubkey,
            });
        });
    });

    describe('Error Handling Edge Cases', () => {
        it('should handle malformed JSON responses', async () => {
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockRejectedValueOnce(new Error('Invalid JSON')),
            });
            await expect(client.getConfig()).rejects.toThrow('Invalid JSON');
        });

        it('should handle responses with an error object', async () => {
            const mockError = { code: -32602, message: 'Invalid params' };
            mockErrorResponse(mockError);
            await expect(client.getConfig()).rejects.toThrow('RPC Error -32602: Invalid params');
        });

        it('should handle empty error object', async () => {
            mockErrorResponse({});
            await expect(client.getConfig()).rejects.toThrow('RPC Error undefined: undefined');
        });
    });

    describe('reCAPTCHA Authentication', () => {
        it('should include x-recaptcha-token header when getRecaptchaToken callback is provided (sync)', async () => {
            const recaptchaClient = new KoraClient({
                getRecaptchaToken: () => 'test-recaptcha-token',
                rpcUrl: mockRpcUrl,
            });

            mockSuccessfulResponse({ version: '1.0.0' });
            await recaptchaClient.getVersion();

            expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
                body: JSON.stringify({
                    id: 1,
                    jsonrpc: '2.0',
                    method: 'getVersion',
                    params: undefined,
                }),
                headers: {
                    'Content-Type': 'application/json',
                    'x-recaptcha-token': 'test-recaptcha-token',
                },
                method: 'POST',
            });
        });

        it('should include x-recaptcha-token header when getRecaptchaToken callback returns Promise', async () => {
            const recaptchaClient = new KoraClient({
                getRecaptchaToken: () => Promise.resolve('async-recaptcha-token'),
                rpcUrl: mockRpcUrl,
            });

            mockSuccessfulResponse({ version: '1.0.0' });
            await recaptchaClient.getVersion();

            expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
                body: JSON.stringify({
                    id: 1,
                    jsonrpc: '2.0',
                    method: 'getVersion',
                    params: undefined,
                }),
                headers: {
                    'Content-Type': 'application/json',
                    'x-recaptcha-token': 'async-recaptcha-token',
                },
                method: 'POST',
            });
        });

        it('should NOT include x-recaptcha-token header when getRecaptchaToken is not provided', async () => {
            mockSuccessfulResponse({ version: '1.0.0' });
            await client.getVersion();

            expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
                body: JSON.stringify({
                    id: 1,
                    jsonrpc: '2.0',
                    method: 'getVersion',
                    params: undefined,
                }),
                headers: {
                    'Content-Type': 'application/json',
                },
                method: 'POST',
            });
        });

        it('should include x-recaptcha-token along with other auth headers', async () => {
            const combinedAuthClient = new KoraClient({
                apiKey: 'test-api-key',
                getRecaptchaToken: () => 'test-recaptcha-token',
                rpcUrl: mockRpcUrl,
            });

            mockSuccessfulResponse({ version: '1.0.0' });
            await combinedAuthClient.getVersion();

            const callArgs = (mockFetch.mock.calls as Array<[string, { headers: Record<string, string> }]>)[0][1];
            expect(callArgs.headers).toMatchObject({
                'Content-Type': 'application/json',
                'x-api-key': 'test-api-key',
                'x-recaptcha-token': 'test-recaptcha-token',
            });
        });

        it('should call getRecaptchaToken callback for each request', async () => {
            let callCount = 0;
            const recaptchaClient = new KoraClient({
                getRecaptchaToken: () => `token-${++callCount}`,
                rpcUrl: mockRpcUrl,
            });

            mockSuccessfulResponse({ version: '1.0.0' });
            await recaptchaClient.getVersion();

            mockSuccessfulResponse({ blockhash: 'test-blockhash' });
            await recaptchaClient.getBlockhash();

            expect(callCount).toBe(2);
            const calls = mockFetch.mock.calls as Array<[string, { headers: Record<string, string> }]>;
            expect(calls[0][1].headers['x-recaptcha-token']).toBe('token-1');
            expect(calls[1][1].headers['x-recaptcha-token']).toBe('token-2');
        });

        it('should propagate errors when getRecaptchaToken callback throws', async () => {
            const recaptchaClient = new KoraClient({
                getRecaptchaToken: () => {
                    throw new Error('reCAPTCHA failed to load');
                },
                rpcUrl: mockRpcUrl,
            });

            await expect(recaptchaClient.getVersion()).rejects.toThrow('reCAPTCHA failed to load');
        });

        it('should propagate errors when getRecaptchaToken returns rejected Promise', async () => {
            const recaptchaClient = new KoraClient({
                getRecaptchaToken: () => Promise.reject(new Error('Token generation failed')),
                rpcUrl: mockRpcUrl,
            });

            await expect(recaptchaClient.getVersion()).rejects.toThrow('Token generation failed');
        });
    });
});

describe('Transaction Utils', () => {
    describe('getInstructionsFromBase64Message', () => {
        it('should parse instructions from a valid base64 message', () => {
            // This is a sample base64 encoded transaction message
            const validMessage =
                'AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQABAwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIDAAEMAgAAAAEAAAAAAAAA';

            const instructions = getInstructionsFromBase64Message(validMessage);

            expect(Array.isArray(instructions)).toBe(true);
            expect(instructions).not.toBeNull();
        });

        it('should return empty array for invalid base64 message', () => {
            const invalidMessage = 'invalid_base64_message';

            const instructions = getInstructionsFromBase64Message(invalidMessage);

            expect(Array.isArray(instructions)).toBe(true);
            expect(instructions).toEqual([]);
        });

        it('should return empty array for empty message', () => {
            const emptyMessage = '';

            const instructions = getInstructionsFromBase64Message(emptyMessage);

            expect(Array.isArray(instructions)).toBe(true);
            expect(instructions).toEqual([]);
        });

        it('should handle malformed transaction messages gracefully', () => {
            // Valid base64 but not a valid transaction message
            const malformedMessage = 'SGVsbG8gV29ybGQh'; // "Hello World!" in base64

            const instructions = getInstructionsFromBase64Message(malformedMessage);

            expect(Array.isArray(instructions)).toBe(true);
            expect(instructions).toEqual([]);
        });
    });
});
