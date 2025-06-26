import { Callable, RpcRequestTransformer, RpcResponse, RpcResponseTransformer } from '@solana/rpc-spec-types';
import type { RpcTransport } from './rpc-transport';
export type RpcApiConfig = Readonly<{
    requestTransformer?: RpcRequestTransformer;
    responseTransformer?: RpcResponseTransformer;
}>;
export type RpcPlan<TResponse> = {
    execute: (config: Readonly<{
        signal?: AbortSignal;
        transport: RpcTransport;
    }>) => Promise<RpcResponse<TResponse>>;
};
export type RpcApi<TRpcMethods> = {
    [MethodName in keyof TRpcMethods]: RpcReturnTypeMapper<TRpcMethods[MethodName]>;
};
type RpcReturnTypeMapper<TRpcMethod> = TRpcMethod extends Callable ? (...rawParams: unknown[]) => RpcPlan<ReturnType<TRpcMethod>> : never;
type RpcApiMethod = (...args: any) => any;
interface RpcApiMethods {
    [methodName: string]: RpcApiMethod;
}
export declare function createJsonRpcApi<TRpcMethods extends RpcApiMethods>(config?: RpcApiConfig): RpcApi<TRpcMethods>;
export {};
//# sourceMappingURL=rpc-api.d.ts.map