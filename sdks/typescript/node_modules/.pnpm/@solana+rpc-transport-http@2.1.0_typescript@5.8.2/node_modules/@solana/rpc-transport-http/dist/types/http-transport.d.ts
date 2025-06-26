import type { RpcTransport } from '@solana/rpc-spec';
import type { RpcResponse } from '@solana/rpc-spec-types';
import type Dispatcher from 'undici-types/dispatcher';
import { AllowedHttpRequestHeaders } from './http-transport-headers';
type Config = Readonly<{
    dispatcher_NODE_ONLY?: Dispatcher;
    fromJson?: (rawResponse: string, payload: unknown) => RpcResponse;
    headers?: AllowedHttpRequestHeaders;
    toJson?: (payload: unknown) => string;
    url: string;
}>;
export declare function createHttpTransport(config: Config): RpcTransport;
export {};
//# sourceMappingURL=http-transport.d.ts.map