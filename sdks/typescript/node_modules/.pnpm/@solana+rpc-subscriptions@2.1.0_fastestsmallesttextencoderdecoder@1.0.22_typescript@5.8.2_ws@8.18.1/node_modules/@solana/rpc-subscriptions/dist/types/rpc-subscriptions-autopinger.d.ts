import type { RpcSubscriptionsChannel } from '@solana/rpc-subscriptions-spec';
type Config<TChannel extends RpcSubscriptionsChannel<unknown, unknown>> = Readonly<{
    abortSignal: AbortSignal;
    channel: TChannel;
    intervalMs: number;
}>;
export declare function getRpcSubscriptionsChannelWithAutoping<TChannel extends RpcSubscriptionsChannel<object, unknown>>({ abortSignal: callerAbortSignal, channel, intervalMs, }: Config<TChannel>): TChannel;
export {};
//# sourceMappingURL=rpc-subscriptions-autopinger.d.ts.map