import { KoraClient } from '../src/client.js';
import {
    Config,
    EstimateTransactionFeeRequest,
    GetBlockhashResponse,
    GetSupportedTokensResponse,
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
        params: any = undefined
    ) => {
        mockSuccessfulResponse(expectedResult);
        const result = await clientMethod();
        expect(result).toEqual(expectedResult);
        expectRpcCall(methodName, params);
    };

    beforeEach(() => {
        client = new KoraClient(mockRpcUrl);
        mockFetch.mockClear();
    });

    afterEach(() => {
        jest.resetAllMocks();
    });

    describe('Constructor', () => {
        it('should create KoraClient instance with provided RPC URL', () => {
            const testUrl = 'https://api.example.com';
            const testClient = new KoraClient(testUrl);
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
                fee_payer: 'test_fee_payer_address',
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
                    },
                    price: {
                        type: 'margin',
                        margin: 0.1,
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

    describe('getSupportedTokens', () => {
        it('should return supported tokens list', async () => {
            const mockResponse: GetSupportedTokensResponse = {
                tokens: ['SOL', 'USDC', 'USDT'],
            };

            await testSuccessfulRpcMethod('getSupportedTokens', () => client.getSupportedTokens(), mockResponse);
        });
    });

    describe('estimateTransactionFee', () => {
        it('should estimate transaction fee', async () => {
            const request: EstimateTransactionFeeRequest = {
                transaction: 'base64_encoded_transaction',
                fee_token: 'SOL',
            };
            const mockResponse: EstimateTransactionFeeResponse = { fee_in_lamports: 5000 };

            await testSuccessfulRpcMethod(
                'estimateTransactionFee',
                () => client.estimateTransactionFee(request),
                mockResponse,
                request
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
            };

            await testSuccessfulRpcMethod(
                'signTransaction',
                () => client.signTransaction(request),
                mockResponse,
                request
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
            };

            await testSuccessfulRpcMethod(
                'signAndSendTransaction',
                () => client.signAndSendTransaction(request),
                mockResponse,
                request
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
            };

            await testSuccessfulRpcMethod(
                'signTransactionIfPaid',
                () => client.signTransactionIfPaid(request),
                mockResponse,
                request
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
            };

            await testSuccessfulRpcMethod(
                'transferTransaction',
                () => client.transferTransaction(request),
                mockResponse,
                request
            );
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
            await expect(client.getConfig()).rejects.toThrow(
                'RPC Error -32602: Invalid params'
            );
        });

        it('should handle empty error object', async () => {
            mockErrorResponse({});
            await expect(client.getConfig()).rejects.toThrow('RPC Error undefined: undefined');
        });
    });
});
