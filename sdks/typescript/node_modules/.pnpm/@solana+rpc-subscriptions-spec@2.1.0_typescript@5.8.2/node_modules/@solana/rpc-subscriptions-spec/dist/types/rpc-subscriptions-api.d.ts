import { Callable, RpcRequest, RpcRequestTransformer } from '@solana/rpc-spec-types';
import { DataPublisher } from '@solana/subscribable';
import { RpcSubscriptionsChannel } from './rpc-subscriptions-channel';
import { RpcSubscriptionsTransportDataEvents } from './rpc-subscriptions-transport';
export type RpcSubscriptionsApiConfig<TApiMethods extends RpcSubscriptionsApiMethods> = Readonly<{
    planExecutor: RpcSubscriptionsPlanExecutor<ReturnType<TApiMethods[keyof TApiMethods]>>;
    requestTransformer?: RpcRequestTransformer;
}>;
type RpcSubscriptionsPlanExecutor<TNotification> = (config: Readonly<{
    channel: RpcSubscriptionsChannel<unknown, unknown>;
    request: RpcRequest;
    signal: AbortSignal;
}>) => Promise<DataPublisher<RpcSubscriptionsTransportDataEvents<TNotification>>>;
export type RpcSubscriptionsPlan<TNotification> = Readonly<{
    /**
     * This method may be called with a newly-opened channel or a pre-established channel.
     */
    execute: (config: Readonly<{
        channel: RpcSubscriptionsChannel<unknown, unknown>;
        signal: AbortSignal;
    }>) => Promise<DataPublisher<RpcSubscriptionsTransportDataEvents<TNotification>>>;
    /**
     * This request is used to uniquely identify the subscription.
     * It typically comes from the method name and parameters of the subscription call,
     * after potentially being transformed by the RPC Subscriptions API.
     */
    request: RpcRequest;
}>;
export type RpcSubscriptionsApi<TRpcSubscriptionMethods> = {
    [MethodName in keyof TRpcSubscriptionMethods]: RpcSubscriptionsReturnTypeMapper<TRpcSubscriptionMethods[MethodName]>;
};
type RpcSubscriptionsReturnTypeMapper<TRpcMethod> = TRpcMethod extends Callable ? (...rawParams: unknown[]) => RpcSubscriptionsPlan<ReturnType<TRpcMethod>> : never;
type RpcSubscriptionsApiMethod = (...args: any) => any;
export interface RpcSubscriptionsApiMethods {
    [methodName: string]: RpcSubscriptionsApiMethod;
}
export declare function createRpcSubscriptionsApi<TRpcSubscriptionsApiMethods extends RpcSubscriptionsApiMethods>(config: RpcSubscriptionsApiConfig<TRpcSubscriptionsApiMethods>): RpcSubscriptionsApi<TRpcSubscriptionsApiMethods>;
export {};
//# sourceMappingURL=rpc-subscriptions-api.d.ts.map