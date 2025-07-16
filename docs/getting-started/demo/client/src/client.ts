import {
    createDefaultRpcTransport,
    createRpc,
    createJsonRpcApi,
    type Rpc,
    type RpcRequest,
} from "@solana/kit";
import { KoraClient } from "./types";

export function createKoraClient(endpoint: string): Rpc<KoraClient> {
    const transport = createDefaultRpcTransport({ url: endpoint });
    const api = createJsonRpcApi<KoraClient>({
        requestTransformer: (request: RpcRequest<any>) => {
            return {
                ...request,
                params: request.params[0] ? request.params[0] : []
            };
        },
    });
    return createRpc({ api, transport });
}