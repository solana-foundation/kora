import {
    type Address,
    type Base64EncodedWireTransaction,
    type Instruction,
    type TransactionSigner,
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
import {
    findAssociatedTokenPda,
    getTransferInstruction,
    parseTransferInstruction,
    TOKEN_PROGRAM_ADDRESS,
    TRANSFER_DISCRIMINATOR,
} from '@solana-program/token';
import { payer } from '@solana/kit-plugin-payer';
import {
    transactionPlanner as transactionPlannerPlugin,
    transactionPlanExecutor as transactionPlanExecutorPlugin,
    planAndSendTransactions,
} from '@solana/kit-plugin-instruction-plan';
import { rpc } from '@solana/kit-plugin-rpc';
import { KoraClient } from './client.js';
import { koraPlugin } from './plugin.js';
import type { KoraKitClientConfig } from './types/index.js';

/**
 * Plugin that adds a `paymentAddress` to the client.
 * This is the address where fee payments are sent. The Kora server always
 * returns a payment address, but when the pricing model is "free", the
 * fee amount will be 0 and the payment instruction is stripped from the
 * transaction.
 */
function koraPaymentAddress(paymentAddress?: Address) {
    return <T extends object>(client: T) => ({
        ...client,
        paymentAddress,
    });
}

function buildComputeBudgetInstructions(config: KoraKitClientConfig): Instruction[] {
    const instructions: Instruction[] = [];
    if (config.computeUnitLimit !== undefined) {
        instructions.push(getSetComputeUnitLimitInstruction({ units: config.computeUnitLimit }));
    }
    if (config.computeUnitPrice !== undefined) {
        instructions.push(getSetComputeUnitPriceInstruction({ microLamports: config.computeUnitPrice }));
    }
    return instructions;
}

/**
 * Builds a placeholder payment transfer instruction with amount=0.
 * This reserves space in the transaction during planning so the packer
 * accounts for the payment instruction. The executor extracts instructions
 * from the planned message, updates the placeholder amount via
 * `updatePaymentInstructionAmount`, and rebuilds the transaction message.
 */
async function buildPlaceholderPaymentInstruction(
    feePayerWallet: TransactionSigner,
    paymentAddress: Address,
    feeToken: Address,
    tokenProgramId?: Address,
): Promise<{ instruction: Instruction; sourceTokenAccount: Address; destinationTokenAccount: Address }> {
    const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;

    const [sourceTokenAccount] = await findAssociatedTokenPda({
        owner: feePayerWallet.address,
        tokenProgram,
        mint: feeToken,
    });

    const [destinationTokenAccount] = await findAssociatedTokenPda({
        owner: paymentAddress,
        tokenProgram,
        mint: feeToken,
    });

    const instruction = getTransferInstruction({
        source: sourceTokenAccount,
        destination: destinationTokenAccount,
        authority: feePayerWallet,
        amount: 0,
    });

    return { instruction, sourceTokenAccount, destinationTokenAccount };
}

/**
 * Checks whether an instruction is the placeholder payment transfer by parsing
 * it as an SPL Transfer and comparing the decoded accounts.
 */
function isPlaceholderPaymentInstruction(
    ix: Instruction,
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    feePayerWallet: TransactionSigner,
    tokenProgramId?: Address,
): boolean {
    const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;
    if (ix.programAddress !== tokenProgram) return false;
    if (ix.data?.[0] !== TRANSFER_DISCRIMINATOR) return false;

    const parsed = parseTransferInstruction(
        ix as Instruction & { accounts: NonNullable<Instruction['accounts']>; data: NonNullable<Instruction['data']> },
    );
    return (
        parsed.accounts.source.address === sourceTokenAccount &&
        parsed.accounts.destination.address === destinationTokenAccount &&
        parsed.accounts.authority.address === feePayerWallet.address &&
        parsed.data.amount === 0n
    );
}

/**
 * Replaces the placeholder payment instruction (amount=0) with a real
 * transfer instruction containing the estimated fee amount.
 * Throws if no matching placeholder is found.
 */
function updatePaymentInstructionAmount(
    instructions: readonly Instruction[],
    feePayerWallet: TransactionSigner,
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    amount: number | bigint,
    tokenProgramId?: Address,
): Instruction[] {
    let replaced = false;
    const result = instructions.map(ix => {
        if (
            !isPlaceholderPaymentInstruction(
                ix,
                sourceTokenAccount,
                destinationTokenAccount,
                feePayerWallet,
                tokenProgramId,
            )
        ) {
            return ix;
        }
        replaced = true;
        return getTransferInstruction({
            source: sourceTokenAccount,
            destination: destinationTokenAccount,
            authority: feePayerWallet,
            amount,
        });
    });

    if (!replaced) {
        throw new Error(
            'Failed to update payment instruction: no matching placeholder transfer instruction found. ' +
                'This is a Kora SDK internal error — the transaction message may have been modified between planning and execution.',
        );
    }

    return result;
}

/**
 * Removes the placeholder payment instruction from the instruction array.
 * Used when fee estimation returns 0 to avoid sending a no-op transfer.
 */
function removePaymentInstruction(
    instructions: readonly Instruction[],
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    feePayerWallet: TransactionSigner,
    tokenProgramId?: Address,
): Instruction[] {
    return instructions.filter(
        ix =>
            !isPlaceholderPaymentInstruction(
                ix,
                sourceTokenAccount,
                destinationTokenAccount,
                feePayerWallet,
                tokenProgramId,
            ),
    );
}

/**
 * The type returned by `createDefaultKoraClient()`.
 *
 * Satisfies Kit plugin interfaces via the composed plugin chain:
 * - `ClientWithPayer` — provided by `payer()` plugin
 * - `ClientWithTransactionPlanning` — provided by `planAndSendTransactions()` plugin
 * - `ClientWithTransactionSending` — provided by `planAndSendTransactions()` plugin
 */
export type KoraKitClient = Awaited<ReturnType<typeof createDefaultKoraClient>>;

/**
 * Creates a Kora Kit client composed from Kit plugins.
 *
 * The returned client satisfies `ClientWithPayer`, `ClientWithTransactionPlanning`,
 * and `ClientWithTransactionSending`, making it composable with Kit program plugins.
 *
 * The client transparently handles:
 * - Setting the Kora fee payer as a NoopSigner
 * - Fetching blockhash from Kora
 * - Estimating fees and resolving payment instruction placeholders
 * - Partial signing and routing through Kora's signAndSendTransaction
 *
 * @example
 * ```typescript
 * import { createDefaultKoraClient } from '@solana/kora';
 * import { address } from '@solana/kit';
 *
 * const client = await createDefaultKoraClient({
 *   endpoint: 'https://kora.example.com',
 *   rpcUrl: 'https://api.mainnet-beta.solana.com',
 *   feeToken: address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
 *   feePayerWallet: userSigner,
 * });
 *
 * // Send a gasless transaction
 * const result = await client.sendTransaction([myInstruction]);
 *
 * // Composable with Kit program plugins via .use()
 * ```
 */
export async function createDefaultKoraClient(config: KoraKitClientConfig) {
    const koraClient = new KoraClient({
        rpcUrl: config.endpoint,
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
    });

    // Fetch payer info at init time
    const { signer_address, payment_address } = await koraClient.getPayerSigner();
    const paymentAddr = payment_address ? address(payment_address) : undefined;
    const payerSigner = createNoopSigner(address(signer_address));

    const computeBudgetIxs = buildComputeBudgetInstructions(config);
    const solanaRpc = createSolanaRpc(config.rpcUrl);

    // When no explicit CU limit, estimate via simulation for optimal fees.
    const hasCuEstimation = config.computeUnitLimit === undefined;
    const resolveProvisoryComputeUnitLimit = hasCuEstimation
        ? estimateAndUpdateProvisoryComputeUnitLimitFactory(estimateComputeUnitLimitFactory({ rpc: solanaRpc }))
        : undefined;

    // Pre-build the placeholder payment instruction and cache the derived ATAs.
    // The placeholder is added in the planner to reserve transaction space, and the
    // executor resolves the real fee amount via `updatePaymentInstructionAmount`.
    const payment = paymentAddr
        ? await buildPlaceholderPaymentInstruction(
              config.feePayerWallet,
              paymentAddr,
              config.feeToken,
              config.tokenProgramId,
          )
        : undefined;

    // --- Kit Planner ---
    const koraTransactionPlanner = createTransactionPlanner({
        createTransactionMessage: () => {
            const allIxs = [...computeBudgetIxs];
            if (payment) {
                allIxs.push(payment.instruction);
            }
            return pipe(
                createTransactionMessage({ version: 0 }),
                m => setTransactionMessageFeePayerSigner(payerSigner, m),
                m => appendTransactionMessageInstructions(allIxs, m),
                m => (hasCuEstimation ? fillProvisorySetComputeUnitLimitInstruction(m) : m),
            );
        },
    });

    // --- Kit Executor ---
    const koraTransactionPlanExecutor = createTransactionPlanExecutor({
        async executeTransactionMessage(_context, transactionMessage) {
            // 1. Get blockhash from Kora and set lifetime.
            // Kora server manages blockhash validity; client sets max to avoid premature expiry checks.
            const { blockhash: bh } = await koraClient.getBlockhash();
            const msgWithLifetime = setTransactionMessageLifetimeUsingBlockhash(
                {
                    blockhash: blockhash(bh),
                    lastValidBlockHeight: BigInt(Number.MAX_SAFE_INTEGER),
                },
                transactionMessage,
            );

            // 2. Resolve provisory CU limit via simulation
            const msgForEstimation = resolveProvisoryComputeUnitLimit
                ? await resolveProvisoryComputeUnitLimit(msgWithLifetime)
                : msgWithLifetime;

            // 3. Serialize for fee estimation
            const prePaymentTx = getBase64EncodedWireTransaction(
                await partiallySignTransactionMessageWithSigners(msgForEstimation),
            );

            let finalTx: Base64EncodedWireTransaction;

            if (payment) {
                // 4. Estimate fee
                const { sourceTokenAccount, destinationTokenAccount } = payment;
                const { fee_in_token } = await koraClient.estimateTransactionFee({
                    transaction: prePaymentTx,
                    fee_token: config.feeToken,
                });

                if (fee_in_token < 0) {
                    throw new Error(
                        `Kora fee estimation returned a negative fee (${fee_in_token}). This indicates a server-side error.`,
                    );
                }

                // Access instructions from the planned message
                const currentIxs = 'instructions' in msgForEstimation ? msgForEstimation.instructions : undefined;

                if (!currentIxs) {
                    throw new Error(
                        'Cannot extract instructions from transaction message. ' +
                            'The message structure may be incompatible with this version of the Kora SDK.',
                    );
                }

                // 5. Resolve final instructions: update placeholder or strip it
                const finalIxs =
                    fee_in_token > 0
                        ? updatePaymentInstructionAmount(
                              currentIxs,
                              config.feePayerWallet,
                              sourceTokenAccount,
                              destinationTokenAccount,
                              fee_in_token,
                              config.tokenProgramId,
                          )
                        : removePaymentInstruction(
                              currentIxs,
                              sourceTokenAccount,
                              destinationTokenAccount,
                              config.feePayerWallet,
                              config.tokenProgramId,
                          );

                // Rebuild message with resolved instructions
                const resolvedMsg = pipe(
                    createTransactionMessage({ version: 0 }),
                    m => setTransactionMessageFeePayerSigner(payerSigner, m),
                    m =>
                        setTransactionMessageLifetimeUsingBlockhash(
                            {
                                blockhash: blockhash(bh),
                                lastValidBlockHeight: BigInt(Number.MAX_SAFE_INTEGER),
                            },
                            m,
                        ),
                    m => appendTransactionMessageInstructions(finalIxs, m),
                );

                finalTx = getBase64EncodedWireTransaction(
                    await partiallySignTransactionMessageWithSigners(resolvedMsg),
                );
            } else {
                // No payment address: send as-is
                finalTx = prePaymentTx;
            }

            // 6. Send via Kora
            const result = await koraClient.signAndSendTransaction({ transaction: finalTx });

            // Extract signature
            if (result.signature) {
                return signature(result.signature);
            }
            const signedTxBytes = getBase64Encoder().encode(result.signed_transaction);
            const decodedTx = getTransactionDecoder().decode(signedTxBytes);
            return getSignatureFromTransaction(decodedTx);
        },
    });

    // --- Compose client via plugin chain ---
    return createEmptyClient()
        .use(rpc(config.rpcUrl))
        .use(koraPlugin({ endpoint: config.endpoint, apiKey: config.apiKey, hmacSecret: config.hmacSecret }))
        .use(payer(payerSigner))
        .use(koraPaymentAddress(paymentAddr))
        .use(transactionPlannerPlugin(koraTransactionPlanner))
        .use(transactionPlanExecutorPlugin(koraTransactionPlanExecutor))
        .use(planAndSendTransactions());
}
