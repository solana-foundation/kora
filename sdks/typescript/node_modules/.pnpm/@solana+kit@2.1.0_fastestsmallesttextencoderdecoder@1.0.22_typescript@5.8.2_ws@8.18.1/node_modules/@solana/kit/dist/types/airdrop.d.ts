import type { Signature } from '@solana/keys';
import type { GetSignatureStatusesApi, RequestAirdropApi, Rpc } from '@solana/rpc';
import type { RpcSubscriptions, SignatureNotificationsApi } from '@solana/rpc-subscriptions';
import { requestAndConfirmAirdrop_INTERNAL_ONLY_DO_NOT_EXPORT } from './airdrop-internal';
type AirdropFunction = (config: Omit<Parameters<typeof requestAndConfirmAirdrop_INTERNAL_ONLY_DO_NOT_EXPORT>[0], 'confirmSignatureOnlyTransaction' | 'rpc'>) => Promise<Signature>;
type AirdropFactoryConfig<TCluster> = {
    rpc: Rpc<GetSignatureStatusesApi & RequestAirdropApi> & {
        '~cluster'?: TCluster;
    };
    rpcSubscriptions: RpcSubscriptions<SignatureNotificationsApi> & {
        '~cluster'?: TCluster;
    };
};
export declare function airdropFactory({ rpc, rpcSubscriptions }: AirdropFactoryConfig<'devnet'>): AirdropFunction;
export declare function airdropFactory({ rpc, rpcSubscriptions }: AirdropFactoryConfig<'mainnet'>): AirdropFunction;
export declare function airdropFactory({ rpc, rpcSubscriptions }: AirdropFactoryConfig<'testnet'>): AirdropFunction;
export {};
//# sourceMappingURL=airdrop.d.ts.map