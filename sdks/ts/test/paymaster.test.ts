import { createDefaultKoraClient, type KoraPaymasterClient } from '../src/paymaster.js';
import { address, createNoopSigner, type Address, signature as kitSignature } from '@solana/kit';

// Mock fetch globally
const mockFetch = jest.fn();
global.fetch = mockFetch;

const MOCK_ENDPOINT = 'http://localhost:8080';
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

function mockRpcError(code: number, message: string) {
    mockFetch.mockResolvedValueOnce({
        json: jest.fn().mockResolvedValueOnce({
            jsonrpc: '2.0',
            id: 1,
            error: { code, message },
        }),
    });
}

describe('createDefaultKoraClient', () => {
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

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            expect(client.payerAddress).toBe(MOCK_PAYER_ADDRESS);
            expect(client.paymentAddress).toBe(MOCK_PAYMENT_ADDRESS);
            expect(client.payerSigner.address).toBe(MOCK_PAYER_ADDRESS);
            // ClientWithPayer: payer is same as payerSigner
            expect(client.payer.address).toBe(MOCK_PAYER_ADDRESS);
            expect(mockFetch).toHaveBeenCalledTimes(1);

            const body = JSON.parse(mockFetch.mock.calls[0][1].body);
            expect(body.method).toBe('getPayerSigner');
        });

        it('should throw if getPayerSigner fails', async () => {
            mockRpcError(-32000, 'Server error');

            await expect(
                createDefaultKoraClient({
                    endpoint: MOCK_ENDPOINT,
                    feeToken: MOCK_FEE_TOKEN,
                    feePayerWallet: MOCK_WALLET,
                }),
            ).rejects.toThrow('RPC Error -32000: Server error');
        });

        it('should expose kora namespace for raw RPC access', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
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

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
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
        let client: KoraPaymasterClient;

        beforeEach(async () => {
            // Mock getPayerSigner for init
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            mockFetch.mockClear();
        });

        it('should call getBlockhash, estimateTransactionFee, and signAndSendTransaction', async () => {
            // Mock getBlockhash
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });

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

            // Returns SuccessfulSingleTransactionPlanResult with Signature in context
            expect(result.status).toBe('successful');
            expect(result.context.signature).toBe(MOCK_SIGNATURE);
            expect(mockFetch).toHaveBeenCalledTimes(3);

            // Verify call order
            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).toEqual(['getBlockhash', 'estimateTransactionFee', 'signAndSendTransaction']);
        });

        it('should skip payment instruction when fee is 0', async () => {
            mockRpcResponse({ blockhash: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi' });

            // Mock estimateTransactionFee with 0 fee
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
            // Verify signAndSendTransaction was attempted
            const calls = mockFetch.mock.calls.map(c => JSON.parse(c[1].body).method);
            expect(calls).toContain('signAndSendTransaction');
        });
    });

    describe('planTransaction', () => {
        let client: KoraPaymasterClient;

        beforeEach(async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
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

            // Returns a transaction message (has version, instructions, feePayer)
            expect(result).toBeDefined();
            expect('version' in result).toBe(true);
            expect('instructions' in result).toBe(true);
            // Should NOT call any RPC methods (planner is local)
            expect(mockFetch).toHaveBeenCalledTimes(0);
        });
    });

    describe('plugin composition', () => {
        it('should support .use() for extending the client', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            // Add a custom plugin
            const extended = client.use(() => ({
                custom: {
                    hello: () => 'world',
                },
            }));

            expect(extended.custom.hello()).toBe('world');
            // Original methods still available
            expect(extended.kora).toBeDefined();
            expect(typeof extended.sendTransaction).toBe('function');
            expect(typeof extended.planTransaction).toBe('function');
        });

        it('should use correct spread order (plugin additions win over client)', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            // Plugin that adds extra property
            const extended = client.use(() => ({
                extra: 42,
            }));

            expect(extended.extra).toBe(42);
            // Original client methods preserved
            expect(extended.payerAddress).toBe(MOCK_PAYER_ADDRESS);
        });
    });

    describe('auth passthrough', () => {
        it('should pass apiKey to underlying KoraClient', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                apiKey: 'test-api-key',
            });

            // The init call should include the API key header
            const headers = mockFetch.mock.calls[0][1].headers;
            expect(headers['x-api-key']).toBe('test-api-key');
        });
    });

    describe('Token-2022 support', () => {
        it('should accept tokenProgramId in config', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const TOKEN_2022_PROGRAM_ADDRESS = 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb' as Address;

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                tokenProgramId: TOKEN_2022_PROGRAM_ADDRESS,
            });

            expect(client).toBeDefined();
            expect(typeof client.sendTransaction).toBe('function');
        });
    });

    describe('compute budget instructions', () => {
        const COMPUTE_BUDGET_PROGRAM = 'ComputeBudget111111111111111111111111111111';
        // SetComputeUnitLimit discriminator = 0x02, SetComputeUnitPrice discriminator = 0x03
        const CU_LIMIT_DISCRIMINATOR = 2;
        const CU_PRICE_DISCRIMINATOR = 3;

        const DUMMY_IX = {
            programAddress: address('11111111111111111111111111111111'),
            accounts: [],
            data: new Uint8Array(4),
        };

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        function getComputeBudgetIxs(planned: { instructions: readonly any[] }) {
            return planned.instructions.filter(
                (ix: { programAddress: string }) => ix.programAddress === COMPUTE_BUDGET_PROGRAM,
            ) as { programAddress: string; data: Uint8Array }[];
        }

        it('should NOT include any ComputeBudget instructions by default', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(0);
        });

        it('should include SetComputeUnitLimit with correct units when computeUnitLimit is set', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                computeUnitLimit: 200_000,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(1);

            const ix = cbIxs[0];
            // discriminator 0x02 = SetComputeUnitLimit
            expect(ix.data[0]).toBe(CU_LIMIT_DISCRIMINATOR);
            // 200_000 in u32 LE = [0x40, 0x0D, 0x03, 0x00]
            const units = new DataView(ix.data.buffer, ix.data.byteOffset).getUint32(1, true);
            expect(units).toBe(200_000);
        });

        it('should include SetComputeUnitPrice with correct microLamports when computeUnitPrice is set', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                computeUnitPrice: 1000n as import('@solana/kit').MicroLamports,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(1);

            const ix = cbIxs[0];
            // discriminator 0x03 = SetComputeUnitPrice
            expect(ix.data[0]).toBe(CU_PRICE_DISCRIMINATOR);
            // 1000 in u64 LE
            const view = new DataView(ix.data.buffer, ix.data.byteOffset);
            const microLamports = view.getBigUint64(1, true);
            expect(microLamports).toBe(1000n);
        });

        it('should include both CU limit and price instructions when both are set', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                computeUnitLimit: 150_000,
                computeUnitPrice: 500n as import('@solana/kit').MicroLamports,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(2);

            // First should be SetComputeUnitLimit
            expect(cbIxs[0].data[0]).toBe(CU_LIMIT_DISCRIMINATOR);
            const units = new DataView(cbIxs[0].data.buffer, cbIxs[0].data.byteOffset).getUint32(1, true);
            expect(units).toBe(150_000);

            // Second should be SetComputeUnitPrice
            expect(cbIxs[1].data[0]).toBe(CU_PRICE_DISCRIMINATOR);
            const microLamports = new DataView(cbIxs[1].data.buffer, cbIxs[1].data.byteOffset).getBigUint64(1, true);
            expect(microLamports).toBe(500n);
        });

        it('should add provisory CU limit (units=0) when rpcUrl is set without explicit computeUnitLimit', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                rpcUrl: 'http://127.0.0.1:8899',
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(1);

            const ix = cbIxs[0];
            // discriminator 0x02 = SetComputeUnitLimit
            expect(ix.data[0]).toBe(CU_LIMIT_DISCRIMINATOR);
            // Provisory limit is 0 (will be resolved via simulation in executor)
            const units = new DataView(ix.data.buffer, ix.data.byteOffset).getUint32(1, true);
            expect(units).toBe(0);
        });

        it('should use explicit computeUnitLimit (not provisory) when both rpcUrl and computeUnitLimit are set', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                rpcUrl: 'http://127.0.0.1:8899',
                computeUnitLimit: 200_000,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            expect(cbIxs).toHaveLength(1);

            const ix = cbIxs[0];
            expect(ix.data[0]).toBe(CU_LIMIT_DISCRIMINATOR);
            // Should be the explicit 200_000, not 0 (provisory)
            const units = new DataView(ix.data.buffer, ix.data.byteOffset).getUint32(1, true);
            expect(units).toBe(200_000);
        });

        it('should add provisory CU limit AND price when rpcUrl + computeUnitPrice are set', async () => {
            mockRpcResponse({
                signer_address: MOCK_PAYER_ADDRESS,
                payment_address: MOCK_PAYMENT_ADDRESS,
            });

            const client = await createDefaultKoraClient({
                endpoint: MOCK_ENDPOINT,
                feeToken: MOCK_FEE_TOKEN,
                feePayerWallet: MOCK_WALLET,
                rpcUrl: 'http://127.0.0.1:8899',
                computeUnitPrice: 2000n as import('@solana/kit').MicroLamports,
            });

            const planned = await client.planTransaction([DUMMY_IX]);
            const cbIxs = getComputeBudgetIxs(planned);
            // Should have price instruction from config + provisory limit from rpcUrl
            expect(cbIxs).toHaveLength(2);

            // Price instruction (from buildComputeBudgetInstructions)
            const priceIx = cbIxs.find(ix => ix.data[0] === CU_PRICE_DISCRIMINATOR);
            expect(priceIx).toBeDefined();
            const microLamports = new DataView(priceIx!.data.buffer, priceIx!.data.byteOffset).getBigUint64(1, true);
            expect(microLamports).toBe(2000n);

            // Provisory CU limit (from fillProvisorySetComputeUnitLimitInstruction)
            const limitIx = cbIxs.find(ix => ix.data[0] === CU_LIMIT_DISCRIMINATOR);
            expect(limitIx).toBeDefined();
            const units = new DataView(limitIx!.data.buffer, limitIx!.data.byteOffset).getUint32(1, true);
            expect(units).toBe(0);
        });
    });
});
