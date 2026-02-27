import {
    type Address,
    type Base64EncodedWireTransaction,
    type Instruction,
    type TransactionSigner,
    type InstructionPlanInput,
    type SingleTransactionPlan,
    type TransactionPlan,
    type TransactionPlanInput,
    type SuccessfulSingleTransactionPlanResult,
    type TransactionPlanResult,
    address,
    blockhash,
    createEmptyClient,
    createNoopSigner,
    createTransactionMessage,
    createTransactionPlanner,
    createTransactionPlanExecutor,
    setTransactionMessageFeePayerSigner,
    setTransactionMessageLifetimeUsingBlockhash,
    appendTransactionMessageInstructions,
    partiallySignTransactionMessageWithSigners,
    getBase64EncodedWireTransaction,
    getBase64Encoder,
    getSignatureFromTransaction,
    getTransactionDecoder,
    parseInstructionPlanInput,
    parseInstructionOrTransactionPlanInput,
    singleTransactionPlan,
    assertIsSingleTransactionPlan,
    assertIsSuccessfulSingleTransactionPlanResult,
    isTransactionPlan,
    isSingleTransactionPlan,
    signature,
    pipe,
    createSolanaRpc,
} from '@solana/kit';
import {
    getSetComputeUnitLimitInstruction,
    getSetComputeUnitPriceInstruction,
    fillProvisorySetComputeUnitLimitInstruction,
    estimateComputeUnitLimitFactory,
    estimateAndUpdateProvisoryComputeUnitLimitFactory,
} from '@solana-program/compute-budget';
import { findAssociatedTokenPda, getTransferInstruction, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';
import { KoraClient } from './client.js';
import { koraPlugin, type KoraApi } from './plugin.js';
import type { KoraPaymasterConfig } from './types/index.js';
import type { ClientWithPayer, ClientWithTransactionPlanning, ClientWithTransactionSending } from '@solana/kit';

/**
 * A Kora paymaster client that implements Kit plugin interfaces for composability
 * with program plugins like `tokenProgram()`.
 *
 * Satisfies:
 * - `ClientWithPayer` — provides `payer` (TransactionSigner)
 * - `ClientWithTransactionPlanning` — provides `planTransaction`, `planTransactions`
 * - `ClientWithTransactionSending` — provides `sendTransaction`, `sendTransactions`
 *
 * Created via `createDefaultKoraClient()`.
 */
export interface KoraPaymasterClient {
    /** Access to the raw Kora RPC methods (Kit-typed) */
    readonly kora: KoraApi;
    /** The Kora payer NoopSigner — satisfies ClientWithPayer */
    readonly payer: TransactionSigner;
    /** Alias for payer */
    readonly payerSigner: TransactionSigner;
    /** The Kora payer address */
    readonly payerAddress: Address;
    /** The payment destination address */
    readonly paymentAddress: Address;
    /** Kit plugin composition */
    use<T extends object>(plugin: (client: KoraPaymasterClient) => T): T & KoraPaymasterClient;
    /**
     * Plan a single transaction from instruction input.
     * Returns the planned transaction message (Kit `SingleTransactionPlan['message']`).
     */
    planTransaction: (
        input: InstructionPlanInput,
        config?: { abortSignal?: AbortSignal },
    ) => Promise<SingleTransactionPlan['message']>;
    /**
     * Plan one or more transactions from instruction input.
     * Returns a full `TransactionPlan`.
     */
    planTransactions: (input: InstructionPlanInput, config?: { abortSignal?: AbortSignal }) => Promise<TransactionPlan>;
    /**
     * Send a single transaction.
     * Accepts instructions, an instruction plan, a single transaction plan, or a transaction message.
     * Handles blockhash, fee estimation, payment injection, signing, and sending via Kora.
     */
    sendTransaction: (
        input: InstructionPlanInput | SingleTransactionPlan | SingleTransactionPlan['message'],
        config?: { abortSignal?: AbortSignal },
    ) => Promise<SuccessfulSingleTransactionPlanResult>;
    /**
     * Send one or more transactions.
     * Accepts instructions, instruction plans, transaction messages, or transaction plans.
     */
    sendTransactions: (
        input: InstructionPlanInput | TransactionPlanInput,
        config?: { abortSignal?: AbortSignal },
    ) => Promise<TransactionPlanResult>;
}

// Compile-time conformance: ensure KoraPaymasterClient satisfies Kit plugin interfaces.
// If Kit changes these interfaces, this will produce a type error at build time.
const _assertPayer: KoraPaymasterClient extends ClientWithPayer ? true : never = true;
const _assertPlanning: KoraPaymasterClient extends ClientWithTransactionPlanning ? true : never = true;
const _assertSending: KoraPaymasterClient extends ClientWithTransactionSending ? true : never = true;

function buildComputeBudgetInstructions(config: KoraPaymasterConfig): Instruction[] {
    const instructions: Instruction[] = [];
    if (config.computeUnitLimit !== undefined) {
        instructions.push(getSetComputeUnitLimitInstruction({ units: config.computeUnitLimit }));
    }
    if (config.computeUnitPrice !== undefined) {
        instructions.push(getSetComputeUnitPriceInstruction({ microLamports: config.computeUnitPrice }));
    }
    return instructions;
}

async function buildPaymentInstruction(
    koraClient: KoraClient,
    config: KoraPaymasterConfig,
    paymentAddr: Address,
    feePayerWalletSigner: TransactionSigner,
    prePaymentTx: string,
): Promise<{ paymentIx: Instruction; feeInToken: number } | { paymentIx: null; feeInToken: 0 }> {
    const tokenProgram = config.tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;

    const { fee_in_token } = await koraClient.estimateTransactionFee({
        transaction: prePaymentTx,
        fee_token: config.feeToken,
    });

    if (fee_in_token < 0) {
        throw new Error(
            `Kora fee estimation returned a negative fee (${fee_in_token}). This indicates a server-side error.`,
        );
    }

    if (fee_in_token === 0) {
        return { paymentIx: null, feeInToken: 0 };
    }

    const [sourceTokenAccount] = await findAssociatedTokenPda({
        owner: feePayerWalletSigner.address,
        tokenProgram: tokenProgram,
        mint: config.feeToken,
    });

    const [destinationTokenAccount] = await findAssociatedTokenPda({
        owner: paymentAddr,
        tokenProgram: tokenProgram,
        mint: config.feeToken,
    });

    const paymentIx: Instruction = getTransferInstruction({
        source: sourceTokenAccount,
        destination: destinationTokenAccount,
        authority: feePayerWalletSigner,
        amount: fee_in_token,
    });

    return { paymentIx, feeInToken: fee_in_token };
}

/**
 * Creates a Kora paymaster client that implements Kit plugin interfaces.
 *
 * The returned client satisfies `ClientWithPayer`, `ClientWithTransactionPlanning`,
 * and `ClientWithTransactionSending`, making it composable with Kit program plugins
 * like `tokenProgram()`.
 *
 * The client transparently handles:
 * - Setting the Kora fee payer as a NoopSigner
 * - Fetching blockhash from Kora
 * - Estimating fees and injecting payment instructions
 * - Partial signing and routing through Kora's signAndSendTransaction
 *
 * @example
 * ```typescript
 * import { createDefaultKoraClient } from '@solana/kora';
 * import { address } from '@solana/kit';
 *
 * const client = await createDefaultKoraClient({
 *   endpoint: 'https://kora.example.com',
 *   feeToken: address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
 *   feePayerWallet: userSigner, // TransactionSigner that authorizes SPL fee payment
 * });
 *
 * // DX Target 1: Simple instruction-based sending
 * const result = await client.sendTransaction([myInstruction]);
 *
 * // DX Target 2: Composable with Kit program plugins
 * // client satisfies ClientWithTransactionPlanning & ClientWithTransactionSending
 * ```
 */
export async function createDefaultKoraClient(config: KoraPaymasterConfig): Promise<KoraPaymasterClient> {
    const koraClient = new KoraClient({
        rpcUrl: config.endpoint,
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
    });

    // Fetch payer info at init time
    const { signer_address, payment_address } = await koraClient.getPayerSigner();
    const payerAddress = address(signer_address);
    const paymentAddr = address(payment_address);
    const payerSigner = createNoopSigner(payerAddress);

    // Build the base client with koraPlugin for .kora namespace
    const baseClient = createEmptyClient().use(
        koraPlugin({
            endpoint: config.endpoint,
            apiKey: config.apiKey,
            hmacSecret: config.hmacSecret,
        }),
    );

    const computeBudgetIxs = buildComputeBudgetInstructions(config);

    // When rpcUrl is provided and no explicit CU limit, create an estimator
    // that resolves provisory CU limits via simulation against a Solana RPC node.
    const hasCuEstimation = config.rpcUrl !== undefined && config.computeUnitLimit === undefined;
    const resolveProvisoryComputeUnitLimit = hasCuEstimation
        ? estimateAndUpdateProvisoryComputeUnitLimitFactory(
              estimateComputeUnitLimitFactory({ rpc: createSolanaRpc(config.rpcUrl!) }),
          )
        : undefined;

    // --- Kit Planner: creates transaction messages from instruction plans ---
    const transactionPlanner = createTransactionPlanner({
        createTransactionMessage: () =>
            pipe(
                createTransactionMessage({ version: 0 }),
                m => setTransactionMessageFeePayerSigner(payerSigner, m),
                m => appendTransactionMessageInstructions(computeBudgetIxs, m),
                // When CU estimation is enabled, add a provisory CU limit instruction (set to 0).
                // This will be resolved to the actual CU limit in the executor via simulation.
                m => (hasCuEstimation ? fillProvisorySetComputeUnitLimitInstruction(m) : m),
            ),
    });

    // --- Kit Executor: signs and sends transaction messages via Kora ---
    const transactionPlanExecutor = createTransactionPlanExecutor({
        async executeTransactionMessage(context, transactionMessage) {
            // 1. Get blockhash from Kora and set lifetime
            const { blockhash: bh } = await koraClient.getBlockhash();
            const msgWithLifetime = setTransactionMessageLifetimeUsingBlockhash(
                {
                    blockhash: blockhash(bh),
                    // Kora server manages blockhash validity; client sets max to avoid premature expiry checks.
                    lastValidBlockHeight: BigInt(Number.MAX_SAFE_INTEGER),
                },
                transactionMessage,
            );

            // 2. Resolve provisory CU limit via simulation (if CU estimation is enabled)
            const msgForEstimation = resolveProvisoryComputeUnitLimit
                ? await resolveProvisoryComputeUnitLimit(msgWithLifetime)
                : msgWithLifetime;

            // 3. Serialize pre-payment for fee estimation
            const prePaymentTx = getBase64EncodedWireTransaction(
                await partiallySignTransactionMessageWithSigners(msgForEstimation),
            );

            // Expose the message on context for debugging/observability
            context.message = msgForEstimation;

            // 4. Estimate fee and maybe inject payment instruction
            const { paymentIx, feeInToken } = await buildPaymentInstruction(
                koraClient,
                config,
                paymentAddr,
                config.feePayerWallet,
                prePaymentTx,
            );

            let finalTx: Base64EncodedWireTransaction;
            if (paymentIx !== null) {
                // Rebuild message with payment IX appended
                const msgWithPayment = appendTransactionMessageInstructions([paymentIx], msgForEstimation);
                const signedTx = await partiallySignTransactionMessageWithSigners(msgWithPayment);
                finalTx = getBase64EncodedWireTransaction(signedTx);
            } else {
                finalTx = prePaymentTx;
            }

            // 5. Send via Kora
            const result = await koraClient.signAndSendTransaction({
                transaction: finalTx,
            });

            // Extract signature: prefer explicit field (future Kora versions),
            // fall back to decoding from the fully-signed transaction.
            if (result.signature) {
                return signature(result.signature);
            }
            const signedTxBytes = getBase64Encoder().encode(result.signed_transaction);
            const decodedTx = getTransactionDecoder().decode(signedTxBytes);
            return getSignatureFromTransaction(decodedTx);
        },
    });

    // --- Build the 4 interface methods ---
    const planTransaction: KoraPaymasterClient['planTransaction'] = async (input, config) => {
        const instructionPlan = parseInstructionPlanInput(input);
        const plan = await transactionPlanner(instructionPlan, config);
        assertIsSingleTransactionPlan(plan);
        return plan.message;
    };

    const planTransactions: KoraPaymasterClient['planTransactions'] = async (input, config) => {
        const instructionPlan = parseInstructionPlanInput(input);
        return transactionPlanner(instructionPlan, config);
    };

    const sendTransaction: KoraPaymasterClient['sendTransaction'] = async (input, config) => {
        let plan: TransactionPlan;

        if (isTransactionPlan(input)) {
            // Already a TransactionPlan (includes SingleTransactionPlan)
            plan = input;
        } else if (
            // Duck-type check for transaction messages: they have `version` but are not instructions
            // (which have `programAddress`) or arrays. Kit does not export an isTransactionMessage guard.
            typeof input === 'object' &&
            input !== null &&
            'version' in input &&
            !('programAddress' in input) &&
            !Array.isArray(input)
        ) {
            plan = singleTransactionPlan(input as SingleTransactionPlan['message']);
        } else {
            // It's InstructionPlanInput — plan it first
            const instructionPlan = parseInstructionPlanInput(input as InstructionPlanInput);
            plan = await transactionPlanner(instructionPlan, config);
        }

        if (!isSingleTransactionPlan(plan)) {
            throw new Error(
                'sendTransaction requires instructions that fit in a single transaction. ' +
                    'Use sendTransactions() for multi-transaction instruction sets.',
            );
        }
        const result = await transactionPlanExecutor(plan, config);
        assertIsSuccessfulSingleTransactionPlanResult(result);
        return result;
    };

    const sendTransactions: KoraPaymasterClient['sendTransactions'] = async (input, config) => {
        let plan: TransactionPlan;

        const parsed = parseInstructionOrTransactionPlanInput(input);
        if (isTransactionPlan(parsed)) {
            plan = parsed;
        } else {
            // It's an InstructionPlan — run through the planner
            plan = await transactionPlanner(parsed, config);
        }

        return transactionPlanExecutor(plan, config);
    };

    // --- Compose the client ---
    const client: KoraPaymasterClient = {
        kora: baseClient.kora,
        payer: payerSigner,
        payerSigner,
        payerAddress,
        paymentAddress: paymentAddr,

        use<T extends object>(plugin: (client: KoraPaymasterClient) => T): T & KoraPaymasterClient {
            const additions = plugin(client);
            return { ...client, ...additions } as T & KoraPaymasterClient;
        },

        planTransaction,
        planTransactions,
        sendTransaction,
        sendTransactions,
    };

    return client;
}
