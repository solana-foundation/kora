import type { Signature } from '@solana/keys';
import type { GetSignatureStatusesApi, Rpc } from '@solana/rpc';
import type { RpcSubscriptions, SignatureNotificationsApi } from '@solana/rpc-subscriptions';
import { type Commitment } from '@solana/rpc-types';
type GetRecentSignatureConfirmationPromiseFn = (config: {
    abortSignal: AbortSignal;
    commitment: Commitment;
    signature: Signature;
}) => Promise<void>;
type CreateRecentSignatureConfirmationPromiseFactoryConfig<TCluster> = {
    rpc: Rpc<GetSignatureStatusesApi> & {
        '~cluster'?: TCluster;
    };
    rpcSubscriptions: RpcSubscriptions<SignatureNotificationsApi> & {
        '~cluster'?: TCluster;
    };
};
export declare function createRecentSignatureConfirmationPromiseFactory({ rpc, rpcSubscriptions, }: CreateRecentSignatureConfirmationPromiseFactoryConfig<'devnet'>): GetRecentSignatureConfirmationPromiseFn;
export declare function createRecentSignatureConfirmationPromiseFactory({ rpc, rpcSubscriptions, }: CreateRecentSignatureConfirmationPromiseFactoryConfig<'testnet'>): GetRecentSignatureConfirmationPromiseFn;
export declare function createRecentSignatureConfirmationPromiseFactory({ rpc, rpcSubscriptions, }: CreateRecentSignatureConfirmationPromiseFactoryConfig<'mainnet'>): GetRecentSignatureConfirmationPromiseFn;
export {};
//# sourceMappingURL=confirmation-strategy-recent-signature.d.ts.map