import { type FetchAccountConfig } from '@solana/accounts';
import { type FixedSizeCodec, type FixedSizeDecoder, type FixedSizeEncoder } from '@solana/codecs';
import type { GetAccountInfoApi } from '@solana/rpc-api';
import type { Rpc } from '@solana/rpc-spec';
import type { Slot } from '@solana/rpc-types';
type SysvarLastRestartSlotSize = 8;
/**
 * The `LastRestartSlot` sysvar.
 *
 * Information about the last restart slot (hard fork).
 *
 * The `LastRestartSlot` sysvar provides access to the last restart slot kept in the
 * bank fork for the slot on the fork that executes the current transaction.
 * In case there was no fork it returns `0`.
 */
export type SysvarLastRestartSlot = Readonly<{
    lastRestartSlot: Slot;
}>;
export declare function getSysvarLastRestartSlotEncoder(): FixedSizeEncoder<SysvarLastRestartSlot, SysvarLastRestartSlotSize>;
export declare function getSysvarLastRestartSlotDecoder(): FixedSizeDecoder<SysvarLastRestartSlot, SysvarLastRestartSlotSize>;
export declare function getSysvarLastRestartSlotCodec(): FixedSizeCodec<SysvarLastRestartSlot, SysvarLastRestartSlot, SysvarLastRestartSlotSize>;
/**
 * Fetch the `LastRestartSlot` sysvar.
 *
 * Information about the last restart slot (hard fork).
 *
 * The `LastRestartSlot` sysvar provides access to the last restart slot kept in the
 * bank fork for the slot on the fork that executes the current transaction.
 * In case there was no fork it returns `0`.
 */
export declare function fetchSysvarLastRestartSlot(rpc: Rpc<GetAccountInfoApi>, config?: FetchAccountConfig): Promise<SysvarLastRestartSlot>;
export {};
//# sourceMappingURL=last-restart-slot.d.ts.map