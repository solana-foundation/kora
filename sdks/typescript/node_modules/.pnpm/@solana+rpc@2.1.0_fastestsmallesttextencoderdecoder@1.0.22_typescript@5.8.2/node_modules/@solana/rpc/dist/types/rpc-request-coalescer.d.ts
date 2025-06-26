import type { RpcTransport } from '@solana/rpc-spec';
type GetDeduplicationKeyFn = (payload: unknown) => string | undefined;
export declare function getRpcTransportWithRequestCoalescing<TTransport extends RpcTransport>(transport: TTransport, getDeduplicationKey: GetDeduplicationKeyFn): TTransport;
export {};
//# sourceMappingURL=rpc-request-coalescer.d.ts.map