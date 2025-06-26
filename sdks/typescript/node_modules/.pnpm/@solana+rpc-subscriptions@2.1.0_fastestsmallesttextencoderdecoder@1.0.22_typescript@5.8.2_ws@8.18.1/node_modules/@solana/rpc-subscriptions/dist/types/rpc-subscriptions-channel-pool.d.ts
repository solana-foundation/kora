import { RpcSubscriptionsChannelCreator } from '@solana/rpc-subscriptions-spec';
type Config = Readonly<{
    maxSubscriptionsPerChannel: number;
    minChannels: number;
}>;
export declare function getChannelPoolingChannelCreator<TChannelCreator extends RpcSubscriptionsChannelCreator<unknown, unknown>>(createChannel: TChannelCreator, { maxSubscriptionsPerChannel, minChannels }: Config): TChannelCreator;
export {};
//# sourceMappingURL=rpc-subscriptions-channel-pool.d.ts.map