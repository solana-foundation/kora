import type { ClusterUrl } from '@solana/rpc-types';
import { RpcSubscriptionsChannelCreatorFromClusterUrl } from './rpc-subscriptions-clusters';
export type DefaultRpcSubscriptionsChannelConfig<TClusterUrl extends ClusterUrl> = Readonly<{
    intervalMs?: number;
    maxSubscriptionsPerChannel?: number;
    minChannels?: number;
    sendBufferHighWatermark?: number;
    url: TClusterUrl;
}>;
export declare function createDefaultSolanaRpcSubscriptionsChannelCreator<TClusterUrl extends ClusterUrl>(config: DefaultRpcSubscriptionsChannelConfig<TClusterUrl>): RpcSubscriptionsChannelCreatorFromClusterUrl<TClusterUrl, unknown, unknown>;
export declare function createDefaultRpcSubscriptionsChannelCreator<TClusterUrl extends ClusterUrl>(config: DefaultRpcSubscriptionsChannelConfig<TClusterUrl>): RpcSubscriptionsChannelCreatorFromClusterUrl<TClusterUrl, unknown, unknown>;
//# sourceMappingURL=rpc-subscriptions-channel.d.ts.map