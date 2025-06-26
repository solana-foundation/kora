import { type FetchAccountConfig } from '@solana/accounts';
import { type FixedSizeCodec, type FixedSizeDecoder, type FixedSizeEncoder } from '@solana/codecs';
import type { GetAccountInfoApi } from '@solana/rpc-api';
import type { Rpc } from '@solana/rpc-spec';
import type { Epoch, Slot, UnixTimestamp } from '@solana/rpc-types';
type SysvarClockSize = 40;
/**
 * The `Clock` sysvar.
 *
 * Information about the network’s clock, ticks, slots, etc.
 */
export type SysvarClock = Readonly<{
    epoch: Epoch;
    epochStartTimestamp: UnixTimestamp;
    leaderScheduleEpoch: Epoch;
    slot: Slot;
    unixTimestamp: UnixTimestamp;
}>;
export declare function getSysvarClockEncoder(): FixedSizeEncoder<SysvarClock, SysvarClockSize>;
export declare function getSysvarClockDecoder(): FixedSizeDecoder<SysvarClock, SysvarClockSize>;
export declare function getSysvarClockCodec(): FixedSizeCodec<SysvarClock, SysvarClock, SysvarClockSize>;
/**
 * Fetch the `Clock` sysvar.
 *
 * Information about the network’s clock, ticks, slots, etc.
 */
export declare function fetchSysvarClock(rpc: Rpc<GetAccountInfoApi>, config?: FetchAccountConfig): Promise<SysvarClock>;
export {};
//# sourceMappingURL=clock.d.ts.map