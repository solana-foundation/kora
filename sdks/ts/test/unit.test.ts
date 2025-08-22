import { KoraClient } from '../src/client.js';
import {
    Config,
    EstimateTransactionFeeRequest,
    GetBlockhashResponse,
    GetSupportedTokensResponse,
    GetPayerSignerResponse,
    SignTransactionRequest,
    SignTransactionResponse,
    SignAndSendTransactionRequest,
    SignAndSendTransactionResponse,
    SignTransactionIfPaidRequest,
    SignTransactionIfPaidResponse,
    TransferTransactionRequest,
    TransferTransactionResponse,
    EstimateTransactionFeeResponse,
} from '../src/types/index.js';

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
                jsonrpc: '2.0',
                id: 1,
                result,
            }),
        });
    };

    const mockErrorResponse = (error: any) => {
        mockFetch.mockResolvedValueOnce({
            json: jest.fn().mockResolvedValueOnce({
                jsonrpc: '2.0',
                id: 1,
                error,
            }),
        });
    };

    const expectRpcCall = (method: string, params: any = undefined) => {
        expect(mockFetch).toHaveBeenCalledWith(mockRpcUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method,
                params,
            }),
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
                fee_payers: ['test_fee_payer_address'],
                validation_config: {
                    max_allowed_lamports: 1000000,
                    max_signatures: 10,
                    price_source: 'Jupiter',
                    allowed_programs: ['program1', 'program2'],
                    allowed_tokens: ['token1', 'token2'],
                    allowed_spl_paid_tokens: ['spl_token1'],
                    disallowed_accounts: ['account1'],
                    fee_payer_policy: {
                        allow_sol_transfers: true,
                        allow_spl_transfers: true,
                        allow_token2022_transfers: false,
                        allow_assign: true,
                        allow_burn: true,
                        allow_close_account: true,
                        allow_approve: true,
                    },
                    price: {
                        type: 'margin',
                        margin: 0.1,
                    },
                    token2022: {
                        blocked_mint_extensions: ['extension1', 'extension2'],
                        blocked_account_extensions: ['account_extension1', 'account_extension2'],
                    },
                },
                enabled_methods: {
                    liveness: true,
                    estimate_transaction_fee: true,
                    get_supported_tokens: true,
                    sign_transaction: true,
                    sign_and_send_transaction: true,
                    transfer_transaction: true,
                    get_blockhash: true,
                    get_config: true,
                    sign_transaction_if_paid: true,
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
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            await testSuccessfulRpcMethod('getPayerSigner', () => client.getPayerSigner(), mockResponse);
        });

        it('should return same address for signer and payment_destination when no separate paymaster', async () => {
            const mockResponse: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            await testSuccessfulRpcMethod('getPayerSigner', () => client.getPayerSigner(), mockResponse);
            expect(mockResponse.signer_address).toBe(mockResponse.payment_address);
        });
    });

    describe('estimateTransactionFee', () => {
        it('should estimate transaction fee', async () => {
            const request: EstimateTransactionFeeRequest = {
                transaction: 'base64_encoded_transaction',
                fee_token: 'SOL',
            };
            const mockResponse: EstimateTransactionFeeResponse = {
                fee_in_lamports: 5000,
                fee_in_token: 25,
                signer_pubkey: 'test_signer_pubkey',
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
                signature: 'test_signature',
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
                signature: 'test_signature',
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

    describe('signTransactionIfPaid', () => {
        const testSignTransactionIfPaid = async (margin?: number) => {
            const request: SignTransactionIfPaidRequest = {
                transaction: 'base64_encoded_transaction',
                ...(margin !== undefined && { margin }),
            };
            const mockResponse: SignTransactionIfPaidResponse = {
                transaction: 'base64_encoded_transaction',
                signed_transaction: 'base64_signed_transaction',
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod(
                'signTransactionIfPaid',
                () => client.signTransactionIfPaid(request),
                mockResponse,
                request,
            );
        };

        it('should sign transaction if paid', () => testSignTransactionIfPaid(10));
        it('should handle request without margin', () => testSignTransactionIfPaid());
    });

    describe('transferTransaction', () => {
        it('should create transfer transaction', async () => {
            const request: TransferTransactionRequest = {
                amount: 1000000,
                token: 'SOL',
                source: 'source_address',
                destination: 'destination_address',
            };
            const mockResponse: TransferTransactionResponse = {
                transaction: 'base64_encoded_transaction',
                message: 'Transfer transaction created',
                blockhash: 'test_blockhash',
                signer_pubkey: 'test_signer_pubkey',
            };

            await testSuccessfulRpcMethod(
                'transferTransaction',
                () => client.transferTransaction(request),
                mockResponse,
                request,
            );
        });
    });
    describe('getPaymentInstruction', () => {
        const mockConfig: Config = {
            fee_payers: ['11111111111111111111111111111111'],
            validation_config: {
                max_allowed_lamports: 1000000,
                max_signatures: 10,
                price_source: 'Jupiter',
                allowed_programs: ['program1'],
                allowed_tokens: ['token1'],
                allowed_spl_paid_tokens: ['4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU'],
                disallowed_accounts: [],
                fee_payer_policy: {
                    allow_sol_transfers: true,
                    allow_spl_transfers: true,
                    allow_token2022_transfers: true,
                    allow_assign: true,
                    allow_burn: true,
                    allow_close_account: true,
                    allow_approve: true,
                },
                price: {
                    type: 'margin',
                    margin: 0.1,
                },
                token2022: {
                    blocked_mint_extensions: [],
                    blocked_account_extensions: [],
                },
            },
            enabled_methods: {
                liveness: true,
                estimate_transaction_fee: true,
                get_supported_tokens: true,
                sign_transaction: true,
                sign_and_send_transaction: true,
                transfer_transaction: true,
                get_blockhash: true,
                get_config: true,
                sign_transaction_if_paid: true,
            },
        };

        const mockFeeEstimate: EstimateTransactionFeeResponse = {
            fee_in_lamports: 5000,
            fee_in_token: 50000,
            signer_pubkey: 'test_signer_pubkey',
        };

        // Create a mock base64-encoded transaction
        // This is a minimal valid transaction structure
        const mockTransactionBase64 =
            'Aoq7ymA5OGP+gmDXiY5m3cYXlY2Rz/a/gFjOgt9ZuoCS7UzuiGGaEnW2OOtvHvMQHkkD7Z4LRF5B63ftu+1oZwIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgECB1urjQEjgFgzqYhJ8IXJeSg4cJP1j1g2CJstOQTDchOKUzqH3PxgGW3c4V3vZV05A5Y30/MggOBs0Kd00s1JEwg5TaEeaV4+KL2y7fXIAuf6cN0ZQitbhY+G9ExtBSChspOXPgNcy9pYpETe4bmB+fg4bfZx1tnicA/kIyyubczAmbcIKIuniNOOQYG2ggKCz8NjEsHVezrWMatndu1wk6J5miGP26J6Vwp31AljiAajAFuP0D9mWJwSeFuA7J5rPwbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpd/O36SW02zRtNtqk6GFeip2+yBQsVTeSbLL4rWJRkd4CBgQCBQQBCgxAQg8AAAAAAAYGBAIFAwEKDBAnAAAAAAAABg==';

        const validRequest = {
            transaction: mockTransactionBase64,
            fee_token: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
            source_wallet: '11111111111111111111111111111111',
            token_program_id: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        };

        beforeEach(() => {
            // Mock console.log to avoid noise in tests
            jest.spyOn(console, 'log').mockImplementation();
        });

        afterEach(() => {
            jest.restoreAllMocks();
        });

        it('should successfully append payment instruction', async () => {
            const mockPayerSigner: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            // Mock getConfig call
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockConfig,
                }),
            });

            // Mock getPayerSigner call
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockPayerSigner,
                }),
            });

            // Mock estimateTransactionFee call
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockFeeEstimate,
                }),
            });

            const result = await client.getPaymentInstruction(validRequest);

            expect(result).toEqual({
                original_transaction: validRequest.transaction,
                payment_instruction: expect.any(Object),
                payment_amount: mockFeeEstimate.fee_in_token,
                payment_token: validRequest.fee_token,
                payment_address: mockPayerSigner.payment_address,
                signer_address: mockPayerSigner.signer_address,
            });

            // Verify getConfig was called
            expect(mockFetch).toHaveBeenNthCalledWith(1, mockRpcUrl, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    jsonrpc: '2.0',
                    id: 1,
                    method: 'getConfig',
                    params: undefined,
                }),
            });

            // Verify getPayerSigner was called
            expect(mockFetch).toHaveBeenNthCalledWith(2, mockRpcUrl, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    jsonrpc: '2.0',
                    id: 1,
                    method: 'getPayerSigner',
                    params: undefined,
                }),
            });

            // Verify estimateTransactionFee was called
            expect(mockFetch).toHaveBeenNthCalledWith(3, mockRpcUrl, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    jsonrpc: '2.0',
                    id: 1,
                    method: 'estimateTransactionFee',
                    params: {
                        transaction: validRequest.transaction,
                        fee_token: validRequest.fee_token,
                    },
                }),
            });
        });

        it('should throw error when token is not supported', async () => {
            const unsupportedTokenConfig = {
                ...mockConfig,
                validation_config: {
                    ...mockConfig.validation_config,
                    allowed_spl_paid_tokens: ['other_token'],
                },
            };

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: unsupportedTokenConfig,
                }),
            });

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow(
                `Token ${validRequest.fee_token} is not supported`,
            );
        });

        it('should throw error when payment is not required (free pricing)', async () => {
            const freeConfig = {
                ...mockConfig,
                validation_config: {
                    ...mockConfig.validation_config,
                    price: {
                        type: 'free',
                    },
                },
            };

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: freeConfig,
                }),
            });

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow(
                'This transaction does not require payment',
            );
        });

        it('should handle fixed pricing configuration', async () => {
            const fixedPriceConfig = {
                ...mockConfig,
                validation_config: {
                    ...mockConfig.validation_config,
                    price: {
                        type: 'fixed',
                        fixed: 100,
                    },
                },
            };

            const mockPayerSigner: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: fixedPriceConfig,
                }),
            });

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockPayerSigner,
                }),
            });

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
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

        it('should handle getConfig RPC error', async () => {
            const mockError = { code: -32601, message: 'Method not found' };
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    error: mockError,
                }),
            });

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow(
                'RPC Error -32601: Method not found',
            );
        });

        it('should handle estimateTransactionFee RPC error', async () => {
            const mockPayerSigner: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            // Mock successful getConfig
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockConfig,
                }),
            });

            // Mock successful getPayerSigner
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockPayerSigner,
                }),
            });

            // Mock failed estimateTransactionFee
            const mockError = { code: -32602, message: 'Invalid transaction' };
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    error: mockError,
                }),
            });

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow(
                'RPC Error -32602: Invalid transaction',
            );
        });

        it('should handle network errors during config fetch', async () => {
            mockFetch.mockRejectedValueOnce(new Error('Network error'));

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow('Network error');
        });

        it('should handle network errors during fee estimation', async () => {
            const mockPayerSigner: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            // Mock successful getConfig
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockConfig,
                }),
            });

            // Mock successful getPayerSigner
            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockPayerSigner,
                }),
            });

            // Mock network error for estimateTransactionFee
            mockFetch.mockRejectedValueOnce(new Error('Fee estimation failed'));

            await expect(client.getPaymentInstruction(validRequest)).rejects.toThrow('Fee estimation failed');
        });

        it('should return correct payment details in response', async () => {
            const mockPayerSigner: GetPayerSignerResponse = {
                signer_address: 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
                payment_address: 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7',
            };

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockConfig,
                }),
            });

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockPayerSigner,
                }),
            });

            mockFetch.mockResolvedValueOnce({
                json: jest.fn().mockResolvedValueOnce({
                    jsonrpc: '2.0',
                    id: 1,
                    result: mockFeeEstimate,
                }),
            });

            const result = await client.getPaymentInstruction(validRequest);

            expect(result).toMatchObject({
                original_transaction: validRequest.transaction,
                payment_instruction: expect.any(Object),
                payment_amount: mockFeeEstimate.fee_in_token,
                payment_token: validRequest.fee_token,
                payment_address: mockPayerSigner.payment_address,
                signer_address: mockPayerSigner.signer_address,
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

    // TODO: Add Authentication Tests (separate PR)
    //
    // describe('Authentication', () => {
    //     describe('API Key Authentication', () => {
    //         - Test that x-api-key header is included when apiKey is provided
    //         - Test requests work without apiKey when not provided
    //         - Test all RPC methods include the header
    //     });
    //
    //     describe('HMAC Authentication', () => {
    //         - Test x-timestamp and x-hmac-signature headers are included when hmacSecret is provided
    //         - Test HMAC signature calculation is correct (SHA256 of timestamp + body)
    //         - Test timestamp is current (within reasonable bounds)
    //         - Test requests work without HMAC when not provided
    //         - Test all RPC methods include the headers
    //     });
    //
    //     describe('Combined Authentication', () => {
    //         - Test both API key and HMAC headers are included when both are provided
    //         - Test headers are correctly combined
    //     });
    // });
});
