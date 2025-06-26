import { RpcRequestTransformer } from '@solana/rpc-spec-types';
import { Commitment } from '@solana/rpc-types';
import { IntegerOverflowHandler } from './request-transformer-integer-overflow';
export type RequestTransformerConfig = Readonly<{
    defaultCommitment?: Commitment;
    onIntegerOverflow?: IntegerOverflowHandler;
}>;
export declare function getDefaultRequestTransformerForSolanaRpc(config?: RequestTransformerConfig): RpcRequestTransformer;
//# sourceMappingURL=request-transformer.d.ts.map