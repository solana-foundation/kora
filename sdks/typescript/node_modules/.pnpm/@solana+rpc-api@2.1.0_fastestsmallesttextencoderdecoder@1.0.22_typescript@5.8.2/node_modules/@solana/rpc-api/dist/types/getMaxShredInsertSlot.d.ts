import type { Slot } from '@solana/rpc-types';
type GetMaxShredInsertSlotApiResponse = Slot;
export type GetMaxShredInsertSlotApi = {
    /**
     * Get the max slot seen from after shred insert.
     * Note that the optional NO_CONFIG object is ignored. See https://github.com/solana-labs/solana-web3.js/issues/1389
     */
    getMaxShredInsertSlot(NO_CONFIG?: Record<string, never>): GetMaxShredInsertSlotApiResponse;
};
export {};
//# sourceMappingURL=getMaxShredInsertSlot.d.ts.map