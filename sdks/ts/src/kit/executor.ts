import {
    type Address,
    appendTransactionMessageInstructions,
    type Base64EncodedWireTransaction,
    blockhash,
    createTransactionMessage,
    createTransactionPlanExecutor,
    getBase64EncodedWireTransaction,
    getBase64Encoder,
    getSignatureFromTransaction,
    getTransactionDecoder,
    type Instruction,
    partiallySignTransactionMessageWithSigners,
    pipe,
    setTransactionMessageFeePayerSigner,
    setTransactionMessageLifetimeUsingBlockhash,
    signature,
    type TransactionMessage,
    type TransactionMessageWithFeePayer,
    type TransactionSigner,
} from '@solana/kit';

import { KoraClient } from '../client.js';
import type { KoraKitClientConfig } from '../types/index.js';
import { removePaymentInstruction, updatePaymentInstructionAmount } from './payment.js';

// TODO: Create a bundle-aware executor (e.g. `createKoraBundlePlanExecutor`) that collects
// multiple planned transaction messages into a single `signAndSendBundle` call instead of
// submitting each one individually via `signAndSendTransaction`. This would let users
// compose Jito bundles through the Kit plan/execute pipeline rather than manually encoding
// transactions and calling `client.kora.signAndSendBundle()`.
export function createKoraTransactionPlanExecutor(
    koraClient: KoraClient,
    config: KoraKitClientConfig,
    payerSigner: TransactionSigner,
    payment: { destinationTokenAccount: Address; sourceTokenAccount: Address } | undefined,
    resolveProvisoryComputeUnitLimit:
        | (<T extends TransactionMessage & TransactionMessageWithFeePayer>(transactionMessage: T) => Promise<T>)
        | undefined,
) {
    return createTransactionPlanExecutor({
        async executeTransactionMessage(_context, transactionMessage) {
            // Kora manages blockhash validity; set max height to avoid premature client-side expiry checks
            const { blockhash: bh } = await koraClient.getBlockhash();
            const msgWithLifetime = setTransactionMessageLifetimeUsingBlockhash(
                {
                    blockhash: blockhash(bh),
                    lastValidBlockHeight: BigInt(Number.MAX_SAFE_INTEGER),
                },
                transactionMessage,
            );

            const msgForEstimation = resolveProvisoryComputeUnitLimit
                ? await resolveProvisoryComputeUnitLimit(msgWithLifetime)
                : msgWithLifetime;

            const prePaymentTx = getBase64EncodedWireTransaction(
                await partiallySignTransactionMessageWithSigners(msgForEstimation),
            );

            let finalTx: Base64EncodedWireTransaction;

            if (payment) {
                const { sourceTokenAccount, destinationTokenAccount } = payment;
                const { fee_in_token } = await koraClient.estimateTransactionFee({
                    fee_token: config.feeToken,
                    transaction: prePaymentTx,
                });

                const feeInToken = fee_in_token ?? 0;

                if (feeInToken < 0) {
                    throw new Error(
                        `Kora fee estimation returned a negative fee (${feeInToken}). This indicates a server-side error.`,
                    );
                }

                const currentIxs =
                    'instructions' in msgForEstimation
                        ? (msgForEstimation as { instructions: readonly Instruction[] }).instructions
                        : undefined;

                if (!currentIxs) {
                    throw new Error(
                        'Cannot extract instructions from transaction message. ' +
                            'The message structure may be incompatible with this version of the Kora SDK.',
                    );
                }

                // Replace placeholder with real fee amount, or strip it if fee is 0
                const finalIxs =
                    feeInToken > 0
                        ? updatePaymentInstructionAmount(
                              currentIxs,
                              config.feePayerWallet,
                              sourceTokenAccount,
                              destinationTokenAccount,
                              feeInToken,
                              config.tokenProgramId,
                          )
                        : removePaymentInstruction(
                              currentIxs,
                              sourceTokenAccount,
                              destinationTokenAccount,
                              config.feePayerWallet,
                              config.tokenProgramId,
                          );

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
                finalTx = prePaymentTx;
            }

            const result = await koraClient.signAndSendTransaction({
                transaction: finalTx,
                user_id: config.userId,
            });

            if (result.signature) {
                return signature(result.signature);
            }
            const signedTxBytes = getBase64Encoder().encode(result.signed_transaction);
            const decodedTx = getTransactionDecoder().decode(signedTxBytes);
            return getSignatureFromTransaction(decodedTx);
        },
    });
}
