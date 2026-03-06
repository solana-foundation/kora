import {
    type Address,
    type Base64EncodedWireTransaction,
    type Instruction,
    type TransactionSigner,
    blockhash,
    createTransactionMessage,
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
} from '@solana/kit';

import { KoraClient } from '../client.js';
import type { KoraKitClientConfig } from '../types/index.js';
import { updatePaymentInstructionAmount, removePaymentInstruction } from './payment.js';

export function createKoraTransactionPlanExecutor(
    koraClient: KoraClient,
    config: KoraKitClientConfig,
    payerSigner: TransactionSigner,
    payment: { sourceTokenAccount: Address; destinationTokenAccount: Address } | undefined,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    resolveProvisoryComputeUnitLimit: ((transactionMessage: any) => Promise<any>) | undefined,
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
                    transaction: prePaymentTx,
                    fee_token: config.feeToken,
                });

                if (fee_in_token < 0) {
                    throw new Error(
                        `Kora fee estimation returned a negative fee (${fee_in_token}). This indicates a server-side error.`,
                    );
                }

                const currentIxs = 'instructions' in msgForEstimation ? msgForEstimation.instructions : undefined;

                if (!currentIxs) {
                    throw new Error(
                        'Cannot extract instructions from transaction message. ' +
                            'The message structure may be incompatible with this version of the Kora SDK.',
                    );
                }

                // Replace placeholder with real fee amount, or strip it if fee is 0
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

            const result = await koraClient.signAndSendTransaction({ transaction: finalTx });

            if (result.signature) {
                return signature(result.signature);
            }
            const signedTxBytes = getBase64Encoder().encode(result.signed_transaction);
            const decodedTx = getTransactionDecoder().decode(signedTxBytes);
            return getSignatureFromTransaction(decodedTx);
        },
    });
}
