import {
    appendTransactionMessageInstructions,
    createTransactionMessage,
    createTransactionPlanner,
    fillTransactionMessageProvisoryResourceLimits,
    type Instruction,
    pipe,
    setTransactionMessageConfig,
    setTransactionMessageFeePayerSigner,
    type TransactionSigner,
} from '@solana/kit';

import type { KoraBundleConfig } from '../types/index.js';

export function createKoraTransactionPlanner(
    payerSigner: TransactionSigner,
    config: KoraBundleConfig,
    paymentInstruction: Instruction | undefined,
) {
    return createTransactionPlanner({
        createTransactionMessage: () =>
            pipe(
                createTransactionMessage({ version: 1 }),
                m => setTransactionMessageFeePayerSigner(payerSigner, m),
                m =>
                    setTransactionMessageConfig(
                        {
                            computeUnitLimit: config.computeUnitLimit,
                            priorityFeeLamports: config.priorityFeeLamports,
                        },
                        m,
                    ),
                m => fillTransactionMessageProvisoryResourceLimits(m),
                m => appendTransactionMessageInstructions(paymentInstruction ? [paymentInstruction] : [], m),
            ),
    });
}
