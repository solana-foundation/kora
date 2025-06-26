import { RpcSubscriptionsChannel } from '@solana/rpc-subscriptions-spec';
export type Config = Readonly<{
    sendBufferHighWatermark: number;
    signal: AbortSignal;
    url: string;
}>;
type WebSocketMessage = ArrayBufferLike | ArrayBufferView | Blob | string;
export declare function createWebSocketChannel({ sendBufferHighWatermark, signal, url, }: Config): Promise<RpcSubscriptionsChannel<WebSocketMessage, string>>;
export {};
//# sourceMappingURL=websocket-channel.d.ts.map