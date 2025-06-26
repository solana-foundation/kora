import type { GetEpochInfoApi, Rpc } from '@solana/rpc';
import type { RpcSubscriptions, SlotNotificationsApi } from '@solana/rpc-subscriptions';
import type { Commitment } from '@solana/rpc-types';
type GetBlockHeightExceedencePromiseFn = (config: {
    abortSignal: AbortSignal;
    commitment?: Commitment;
    lastValidBlockHeight: bigint;
}) => Promise<void>;
type CreateBlockHeightExceedencePromiseFactoryyConfig<TCluster> = {
    rpc: Rpc<GetEpochInfoApi> & {
        '~cluster'?: TCluster;
    };
    rpcSubscriptions: RpcSubscriptions<SlotNotificationsApi> & {
        '~cluster'?: TCluster;
    };
};
export declare function createBlockHeightExceedencePromiseFactory({ rpc, rpcSubscriptions, }: CreateBlockHeightExceedencePromiseFactoryyConfig<'devnet'>): GetBlockHeightExceedencePromiseFn;
export declare function createBlockHeightExceedencePromiseFactory({ rpc, rpcSubscriptions, }: CreateBlockHeightExceedencePromiseFactoryyConfig<'testnet'>): GetBlockHeightExceedencePromiseFn;
export declare function createBlockHeightExceedencePromiseFactory({ rpc, rpcSubscriptions, }: CreateBlockHeightExceedencePromiseFactoryyConfig<'mainnet'>): GetBlockHeightExceedencePromiseFn;
export {};
//# sourceMappingURL=confirmation-strategy-blockheight.d.ts.map