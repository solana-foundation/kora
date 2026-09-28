import { createKitKoraClient, kora, type KoraKitClient } from '../src/kit/index.js';
import {
    AccountRole,
    address,
    createClient,
    createNoopSigner,
    decompileTransactionMessage,
    getBase64Encoder,
    getCompiledTransactionMessageDecoder,
    getTransactionDecoder,
    type Address,
    type Base64EncodedWireTransaction,
    type SignatureBytes,
    type SignatureDictionary,
    type Transaction,
    type TransactionSigner,
    signature as kitSignature,
} from '@solana/kit';
import { identity, signer } from '@solana/kit-plugin-signer';

const mockFetch = jest.fn();
global.fetch = mockFetch;

const MOCK_ENDPOINT = 'http://localhost:8080';
const MOCK_RPC_URL = 'http://127.0.0.1:8899';
const MOCK_PAYER_ADDRESS = 'DemoKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7';
const MOCK_PAYMENT_ADDRESS = 'PayKMZWkk483QoFPLRPQ2XVKB7bWnuXwSjvDE1JsWk7';
const MOCK_FEE_TOKEN = '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU' as Address;
const MOCK_WALLET_ADDRESS = 'BrEe1Xjy2Ky72doGBAhyUPCxMm5b4bRTm3AD6MNMfKmq' as Address;
const MOCK_WALLET = createNoopSigner(MOCK_WALLET_ADDRESS);
const MOCK_SIGNATURE = '5wBzExmp8yR5M6m4KjV8WT9T6B1NMQkaMbsFWqBoDPBMYWxDx6EuSGxNqKfXnBhDhAkEqMiGRjEwKnGhSN3pi3n';

function mockRpcResponse(result: unknown) {
    mockFetch.mockResolvedValueOnce({
        json: jest.fn().mockResolvedValueOnce({
            jsonrpc: '2.0',
            id: 1,
            result,
        }),
    });
}

function mockSimulateResponse(unitsConsumed = 50000, loadedAccountsDataSize = 600_000) {
    const body = JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        result: {
            context: { slot: 1 },
            value: { err: null, loadedAccountsDataSize, logs: [], unitsConsumed },
        },
    });
    mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: new Headers({ 'content-type': 'application/json' }),
        text: jest.fn().mockResolvedValueOnce(body),
        json: jest.fn().mockResolvedValueOnce(JSON.parse(body)),
    });
}

function mockRpcError(code: number, message: string) {
    mockFetch.mockResolvedValueOnce({
        json: jest.fn().mockResolvedValueOnce({
            jsonrpc: '2.0',
            id: 1,
            error: { code, message },
        }),
    });
}

describe('createKitKoraClient', () => {
    beforeEach(() => {
        mockFetch.mockClear();
    });

    afterEach(() => {
        jest.resetAllMocks();
    });

    describe('initialization', () => {
        it('should fetch payer info on creation', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            expect(client.paymentAddress).toBe(MOCK_PAYMENT_ADDRESS);
            // ClientWithPayer: payer is a NoopSigner for the Kora fee payer
            expect(client.payer.address).toBe(MOCK_PAYER_ADDRESS);
            expect(mockFetch).toHaveBeenCalledTimes(1);

            const body = JSON.parse(mockFetch.mock.calls[0][1].body);
            expect(body.method).toBe('getPayerSigner');
        });

        it('should throw if getPayerSigner fails', async () => {
            mockRpcError(-32000, 'Server error');

            await expect(
                createKitKoraClient({
                    endpoint: MOCK_ENDPOINT,
                    rpcUrl: MOCK_RPC_URL,
                    feeToken: MOCK_FEE_TOKEN,
                    feePayerWallet: MOCK_WALLET,
                }),
            ).rejects.toThrow('Kora Error -32000: Server error');
        });

        it('should expose kora namespace for raw RPC access', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            expect(client.kora).toBeDefined();
            expect(typeof client.kora.getConfig).toBe('function');
            expect(typeof client.kora.getBlockhash).toBe('function');
            expect(typeof client.kora.estimateTransactionFee).toBe('function');
        });

        it('should implement Kit plugin interfaces', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            // ClientWithPayer
            expect(client.payer).toBeDefined();
            expect(client.payer.address).toBe(MOCK_PAYER_ADDRESS);
            // ClientWithTransactionPlanning
            expect(typeof client.planTransaction).toBe('function');
            expect(typeof client.planTransactions).toBe('function');
            // ClientWithTransactionSending
            expect(typeof client.sendTransaction).toBe('function');
            expect(typeof client.sendTransactions).toBe('function');
        });
    });

    describe('sendTransaction', () => {
        let client: KoraKitClient;

        beforeEach(async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            mockFetch.mockClear();
        });

        it('should call getBlockhash, simulateTransaction, estimateTransactionFee, and signAndSendTransaction', async () => {
            // Mock getBlockhash
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            // Mock simulateTransaction (CU estimation)
            mockSimulateResponse();
            // Mock estimateTransactionFee
            mockRpcResponse({
                fee_in_lamports: 5000,
                fee_in_token: 50000,
                signer_pubkey: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });
            // Mock signAndSendTransaction
            mockRpcResponse({
                signature: MOCK_SIGNATURE,
                signed_transaction: 'base64signedtx',
                signer_pubkey: MOCK_PAYER_ADDRESS,
            });

            const dummyIx = {
                programAddress: address('11111111111111111111111111111111'),
                accounts: [],
                data: new Uint8Array(4),
            };

            const result = await client.sendTransaction([dummyIx]);

            expect(result.status).toBe('successful');
            expect(result.context.signature).toBe(MOCK_SIGNATURE);
            expect(mockFetch).toHaveBeenCalledTimes(4);

            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).toEqual([
                'getBlockhash',
                'simulateTransaction',
                'estimateTransactionFee',
                'signAndSendTransaction',
            ]);
        });

        it('should submit a v1 transaction carrying the simulated resource limits', async () => {
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse(50_000, 600_000);
            mockRpcResponse({
                fee_in_lamports: 5000,
                fee_in_token: 50000,
                signer_pubkey: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });
            mockRpcResponse({
                signature: MOCK_SIGNATURE,
                signed_transaction: 'base64signedtx',
                signer_pubkey: MOCK_PAYER_ADDRESS,
            });

            await client.sendTransaction([
                {
                    programAddress: address('11111111111111111111111111111111'),
                    accounts: [],
                    data: new Uint8Array(4),
                },
            ]);

            const sendCall = mockFetch.mock.calls.find(c => JSON.parse(c[1].body).method === 'signAndSendTransaction');
            const wire = JSON.parse(sendCall![1].body).params.transaction as string;
            const tx = getTransactionDecoder().decode(getBase64Encoder().encode(wire));
            const message = decompileTransactionMessage(getCompiledTransactionMessageDecoder().decode(tx.messageBytes));

            expect(message.version).toBe(1);
            const config = (message as { config?: { computeUnitLimit?: number; loadedAccountsDataSizeLimit?: number } })
                .config;
            expect(config?.computeUnitLimit).toBeGreaterThanOrEqual(50_000);
            expect(config?.loadedAccountsDataSizeLimit).toBeGreaterThanOrEqual(600_000);
        });

        it('should skip payment instruction when fee is 0', async () => {
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse();

            mockRpcResponse({
                fee_in_lamports: 0,
                fee_in_token: 0,
                signer_pubkey: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            // Mock signAndSendTransaction
            mockRpcResponse({
                signature: MOCK_SIGNATURE,
                signed_transaction: 'base64signedtx',
                signer_pubkey: MOCK_PAYER_ADDRESS,
            });

            const dummyIx = {
                programAddress: address('11111111111111111111111111111111'),
                accounts: [],
                data: new Uint8Array(4),
            };

            const result = await client.sendTransaction([dummyIx]);
            expect(result.status).toBe('successful');
            expect(result.context.signature).toBe(MOCK_SIGNATURE);
        });

        it('should propagate fee estimation errors', async () => {
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse();
            mockRpcError(-32602, 'Invalid transaction');

            const dummyIx = {
                programAddress: address('11111111111111111111111111111111'),
                accounts: [],
                data: new Uint8Array(4),
            };

            // Kit's executor wraps errors — the original RPC error is in the cause chain
            await expect(client.sendTransaction([dummyIx])).rejects.toThrow();
            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).toContain('estimateTransactionFee');
        });

        it('should propagate signAndSendTransaction errors', async () => {
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse();
            mockRpcResponse({
                fee_in_lamports: 5000,
                fee_in_token: 50000,
                signer_pubkey: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });
            mockRpcError(-32003, 'Transaction failed');

            const dummyIx = {
                programAddress: address('11111111111111111111111111111111'),
                accounts: [],
                data: new Uint8Array(4),
            };

            // Kit's executor wraps errors — the original RPC error is the cause
            await expect(client.sendTransaction([dummyIx])).rejects.toThrow();
            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).toContain('signAndSendTransaction');
        });
    });

    describe('planTransaction', () => {
        let client: KoraKitClient;

        beforeEach(async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            mockFetch.mockClear();
        });

        it('should return a transaction message without sending', async () => {
            const dummyIx = {
                programAddress: address('11111111111111111111111111111111'),
                accounts: [],
                data: new Uint8Array(4),
            };

            const result = await client.planTransaction([dummyIx]);

            expect(result).toBeDefined();
            expect('version' in result).toBe(true);
            expect('instructions' in result).toBe(true);
            // Should NOT call any RPC methods (planner is local)
            expect(mockFetch).toHaveBeenCalledTimes(0);
        });
    });

    describe('plugin composition', () => {
        it('should support .use() for extending the client with a Kit plugin', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            // Kit plugins must spread the client to preserve existing properties
            const extended = client.use(<T extends object>(c: T) => ({
                ...c,
                custom: {
                    hello: () => 'world',
                },
            }));

            expect(extended.custom.hello()).toBe('world');
            expect(extended.kora).toBeDefined();
            expect(typeof extended.sendTransaction).toBe('function');
            expect(typeof extended.planTransaction).toBe('function');
        });

        it('should preserve existing properties when extending via plugin spread', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            const extended = client.use(<T extends object>(c: T) => ({
                ...c,
                extra: 42,
            }));

            expect(extended.extra).toBe(42);
            expect(extended.payer.address).toBe(MOCK_PAYER_ADDRESS);
            expect(extended.paymentAddress).toBe(MOCK_PAYMENT_ADDRESS);
        });
    });

    describe('auth passthrough', () => {
        it('should pass apiKey to underlying KoraClient', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                apiKey: 'test-api-key',
            });

            const headers = mockFetch.mock.calls[0][1].headers;
            expect(headers['x-api-key']).toBe('test-api-key');
        });

        it('should pass getRecaptchaToken to underlying KoraClient', async () => {
            const mockGetToken = jest.fn().mockResolvedValue('test-recaptcha-token');

            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                getRecaptchaToken: mockGetToken,
            });

            expect(mockGetToken).toHaveBeenCalledTimes(1);
            const headers = mockFetch.mock.calls[0][1].headers;
            expect(headers['x-recaptcha-token']).toBe('test-recaptcha-token');
        });
    });

    describe('Token-2022 support', () => {
        it('should accept tokenProgramId in config', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const TOKEN_2022_PROGRAM_ADDRESS = 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb' as Address;

            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                tokenProgramId: TOKEN_2022_PROGRAM_ADDRESS,
            });

            expect(client).toBeDefined();
            expect(typeof client.sendTransaction).toBe('function');
        });
    });

    describe('v1 message config', () => {
        const COMPUTE_BUDGET_PROGRAM = 'ComputeBudget111111111111111111111111111111';

        const DUMMY_IX = {
            programAddress: address('11111111111111111111111111111111'),
            accounts: [],
            data: new Uint8Array(4),
        };

        async function planWith(overrides: { computeUnitLimit?: number; priorityFeeLamports?: bigint } = {}) {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });
            const client = await createKitKoraClient({
                endpoint: MOCK_ENDPOINT,
                rpcUrl: MOCK_RPC_URL,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                ...overrides,
            });
            return (await client.planTransaction([DUMMY_IX])) as unknown as {
                config?: Record<string, unknown>;
                instructions: readonly { programAddress: string }[];
                version: number;
            };
        }

        it('builds a v1 message with provisory resource limits and no ComputeBudget instructions', async () => {
            const planned = await planWith();
            expect(planned.version).toBe(1);
            expect(planned.config).toEqual({ computeUnitLimit: 0, loadedAccountsDataSizeLimit: 0 });
            expect(planned.instructions.some(ix => ix.programAddress === COMPUTE_BUDGET_PROGRAM)).toBe(false);
        });

        it('writes an explicit computeUnitLimit and priorityFeeLamports into the config', async () => {
            const planned = await planWith({ computeUnitLimit: 150_000, priorityFeeLamports: 10_000n });
            expect(planned.config).toEqual({
                computeUnitLimit: 150_000,
                loadedAccountsDataSizeLimit: 0,
                priorityFeeLamports: 10_000n,
            });
            expect(planned.instructions.some(ix => ix.programAddress === COMPUTE_BUDGET_PROGRAM)).toBe(false);
        });
    });
});

describe('kora() bundle plugin', () => {
    beforeEach(() => {
        mockFetch.mockClear();
    });

    afterEach(() => {
        jest.resetAllMocks();
    });

    it('composes with identity() and exposes the same surface as the wrapper', async () => {
        mockRpcResponse({
            signer_address: MOCK_PAYER_ADDRESS,
            payment_address: MOCK_PAYMENT_ADDRESS,
        });

        const client = await createClient()
            .use(identity(MOCK_WALLET))
            .use(
                kora({
                    endpoint: MOCK_ENDPOINT,
                    rpcUrl: MOCK_RPC_URL,
                    feeToken: MOCK_FEE_TOKEN,
                }),
            );

        expect(client.identity.address).toBe(MOCK_WALLET_ADDRESS);
        expect(client.payer.address).toBe(MOCK_PAYER_ADDRESS);
        expect(client.paymentAddress).toBe(MOCK_PAYMENT_ADDRESS);
        expect(typeof client.sendTransaction).toBe('function');
        expect(typeof client.planTransaction).toBe('function');
        expect(client.kora).toBeDefined();
    });

    describe('user signature collection', () => {
        // A TransactionPartialSigner that produces real signature bytes and counts how many
        // times the wallet would be asked to sign.
        function createCountingSigner(addr: Address) {
            const countingSigner = {
                address: addr,
                signCount: 0,
                async signTransactions(transactions: readonly Transaction[]): Promise<SignatureDictionary[]> {
                    countingSigner.signCount += 1;
                    return transactions.map(() => ({ [addr]: new Uint8Array(64).fill(7) as SignatureBytes }));
                },
            };
            return countingSigner;
        }

        function decodeWire(wire: string) {
            return getTransactionDecoder().decode(getBase64Encoder().encode(wire as Base64EncodedWireTransaction));
        }

        function filledSignatures(tx: Transaction) {
            return Object.entries(tx.signatures).filter(
                ([, sig]) => sig != null && sig.some((byte: number) => byte !== 0),
            );
        }

        function wireSentTo(method: string) {
            const call = mockFetch.mock.calls.find(c => JSON.parse(c[1].body).method === method);
            expect(call).toBeDefined();
            return JSON.parse(call![1].body).params.transaction as string;
        }

        const DUMMY_IX = {
            programAddress: address('11111111111111111111111111111111'),
            accounts: [],
            data: new Uint8Array(4),
        };

        it('signs exactly once on a paid send, with an unsigned estimation wire', async () => {
            const countingSigner = createCountingSigner(MOCK_WALLET_ADDRESS);
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createClient()
                .use(identity(countingSigner as unknown as TransactionSigner))
                .use(
                    kora({
                        endpoint: MOCK_ENDPOINT,
                        rpcUrl: MOCK_RPC_URL,
                        feeToken: MOCK_FEE_TOKEN,
                    }),
                );

            mockFetch.mockClear();
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse();
            mockRpcResponse({
                fee_in_lamports: 5000,
                fee_in_token: 50000,
                signer_pubkey: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });
            mockRpcResponse({
                signature: MOCK_SIGNATURE,
                signed_transaction: 'base64signedtx',
                signer_pubkey: MOCK_PAYER_ADDRESS,
            });

            const result = await client.sendTransaction([DUMMY_IX]);
            expect(result.status).toBe('successful');

            expect(countingSigner.signCount).toBe(1);

            const estimationTx = decodeWire(wireSentTo('estimateTransactionFee'));
            expect(filledSignatures(estimationTx)).toHaveLength(0);

            const submittedTx = decodeWire(wireSentTo('signAndSendTransaction'));
            const filled = filledSignatures(submittedTx);
            expect(filled.map(([addr]) => addr)).toEqual([MOCK_WALLET_ADDRESS]);
        });

        it('submits a signed transaction on an unpaid send (no payment address)', async () => {
            const countingSigner = createCountingSigner(MOCK_WALLET_ADDRESS);
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: null,
            });

            const client = await createClient()
                .use(identity(countingSigner as unknown as TransactionSigner))
                .use(
                    kora({
                        endpoint: MOCK_ENDPOINT,
                        rpcUrl: MOCK_RPC_URL,
                        feeToken: MOCK_FEE_TOKEN,
                    }),
                );

            mockFetch.mockClear();
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });
            mockSimulateResponse();
            mockRpcResponse({
                signature: MOCK_SIGNATURE,
                signed_transaction: 'base64signedtx',
                signer_pubkey: MOCK_PAYER_ADDRESS,
            });

            const userSignedIx = {
                ...DUMMY_IX,
                accounts: [
                    {
                        address: MOCK_WALLET_ADDRESS,
                        role: AccountRole.WRITABLE_SIGNER,
                        signer: countingSigner as unknown as TransactionSigner,
                    },
                ],
            };
            const result = await client.sendTransaction([userSignedIx]);
            expect(result.status).toBe('successful');

            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).not.toContain('estimateTransactionFee');

            expect(countingSigner.signCount).toBe(1);
            const submittedTx = decodeWire(wireSentTo('signAndSendTransaction'));
            const filled = filledSignatures(submittedTx);
            expect(filled.map(([addr]) => addr)).toEqual([MOCK_WALLET_ADDRESS]);
        });
    });

    // TODO: re-enable once @solana/kit supports overriding an already-set field on an
    // extended client.
    it.skip('overrides client.payer when the caller used signer() (sets identity + payer)', async () => {
        mockRpcResponse({
            signer_address: MOCK_PAYER_ADDRESS,
            payment_address: MOCK_PAYMENT_ADDRESS,
        });

        const client = await createClient()
            .use(signer(MOCK_WALLET))
            .use(
                kora({
                    endpoint: MOCK_ENDPOINT,
                    rpcUrl: MOCK_RPC_URL,
                    feeToken: MOCK_FEE_TOKEN,
                }),
            );

        expect(client.identity.address).toBe(MOCK_WALLET_ADDRESS);
        expect(client.payer.address).toBe(MOCK_PAYER_ADDRESS);
    });
});
