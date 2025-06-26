import type { Commitment, Slot } from '@solana/rpc-types';
type GetTransactionCountApiResponse = bigint;
export type GetTransactionCountApi = {
    /**
     * Returns the current Transaction count from the ledger
     */
    getTransactionCount(config?: Readonly<{
        commitment?: Commitment;
        minContextSlot?: Slot;
    }>): GetTransactionCountApiResponse;
};
export {};
//# sourceMappingURL=getTransactionCount.d.ts.map