import {
    appendTransactionMessageInstructions,
    createTransactionMessage,
    createTransactionPlanner,
    type Instruction,
    pipe,
    setTransactionMessageFeePayerSigner,
    type TransactionSigner,
} from '@solana/kit';
import {
    fillProvisorySetComputeUnitLimitInstruction,
    getSetComputeUnitLimitInstruction,
    getSetComputeUnitPriceInstruction,
} from '@solana-program/compute-budget';

import type { KoraKitClientConfig } from '../types/index.js';

export function buildComputeBudgetInstructions(config: KoraKitClientConfig): Instruction[] {
    const instructions: Instruction[] = [];
    if (config.computeUnitLimit !== undefined) {
        instructions.push(getSetComputeUnitLimitInstruction({ units: config.computeUnitLimit }));
    }
    if (config.computeUnitPrice !== undefined) {
        instructions.push(getSetComputeUnitPriceInstruction({ microLamports: config.computeUnitPrice }));
    }
    return instructions;
}

export function createKoraTransactionPlanner(
    payerSigner: TransactionSigner,
    computeBudgetIxs: Instruction[],
    paymentInstruction: Instruction | undefined,
    hasCuEstimation: boolean,
) {
    return createTransactionPlanner({
        createTransactionMessage: () => {
            const allIxs = [...computeBudgetIxs];
            if (paymentInstruction) {
                allIxs.push(paymentInstruction);
            }
            return pipe(
                createTransactionMessage({ version: 0 }),
                m => setTransactionMessageFeePayerSigner(payerSigner, m),
                m => appendTransactionMessageInstructions(allIxs, m),
                m => (hasCuEstimation ? fillProvisorySetComputeUnitLimitInstruction(m) : m),
            );
        },
    });
}
