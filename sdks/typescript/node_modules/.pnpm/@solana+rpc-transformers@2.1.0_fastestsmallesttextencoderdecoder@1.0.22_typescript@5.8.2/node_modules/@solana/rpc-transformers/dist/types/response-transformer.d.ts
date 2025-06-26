import { RpcResponseTransformer } from '@solana/rpc-spec-types';
import { AllowedNumericKeypaths } from './response-transformer-allowed-numeric-values';
export type ResponseTransformerConfig<TApi> = Readonly<{
    allowedNumericKeyPaths?: AllowedNumericKeypaths<TApi>;
}>;
export declare function getDefaultResponseTransformerForSolanaRpc<TApi>(config?: ResponseTransformerConfig<TApi>): RpcResponseTransformer;
export declare function getDefaultResponseTransformerForSolanaRpcSubscriptions<TApi>(config?: ResponseTransformerConfig<TApi>): RpcResponseTransformer;
//# sourceMappingURL=response-transformer.d.ts.map