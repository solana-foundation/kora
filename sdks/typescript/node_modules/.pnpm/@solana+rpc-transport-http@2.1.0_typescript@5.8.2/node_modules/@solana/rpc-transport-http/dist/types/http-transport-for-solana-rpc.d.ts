import { RpcTransport } from '@solana/rpc-spec';
import type Dispatcher from 'undici-types/dispatcher';
import { AllowedHttpRequestHeaders } from './http-transport-headers';
type Config = Readonly<{
    dispatcher_NODE_ONLY?: Dispatcher;
    headers?: AllowedHttpRequestHeaders;
    url: string;
}>;
export declare function createHttpTransportForSolanaRpc(config: Config): RpcTransport;
export {};
//# sourceMappingURL=http-transport-for-solana-rpc.d.ts.map