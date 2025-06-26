import { RpcSubscriptionsApi, RpcSubscriptionsApiMethods } from '@solana/rpc-subscriptions-spec';
import { RequestTransformerConfig } from '@solana/rpc-transformers';
import { AccountNotificationsApi } from './account-notifications';
import { BlockNotificationsApi } from './block-notifications';
import { LogsNotificationsApi } from './logs-notifications';
import { ProgramNotificationsApi } from './program-notifications';
import { RootNotificationsApi } from './root-notifications';
import { SignatureNotificationsApi } from './signature-notifications';
import { SlotNotificationsApi } from './slot-notifications';
import { SlotsUpdatesNotificationsApi } from './slots-updates-notifications';
import { VoteNotificationsApi } from './vote-notifications';
export type SolanaRpcSubscriptionsApi = AccountNotificationsApi & LogsNotificationsApi & ProgramNotificationsApi & RootNotificationsApi & SignatureNotificationsApi & SlotNotificationsApi;
export type SolanaRpcSubscriptionsApiUnstable = BlockNotificationsApi & SlotsUpdatesNotificationsApi & VoteNotificationsApi;
export type { AccountNotificationsApi, BlockNotificationsApi, LogsNotificationsApi, ProgramNotificationsApi, RootNotificationsApi, SignatureNotificationsApi, SlotNotificationsApi, SlotsUpdatesNotificationsApi, VoteNotificationsApi, };
type Config = RequestTransformerConfig;
export declare function createSolanaRpcSubscriptionsApi<TApi extends RpcSubscriptionsApiMethods = SolanaRpcSubscriptionsApi>(config?: Config): RpcSubscriptionsApi<TApi>;
export declare function createSolanaRpcSubscriptionsApi_UNSTABLE(config?: Config): RpcSubscriptionsApi<AccountNotificationsApi & LogsNotificationsApi & ProgramNotificationsApi & RootNotificationsApi & SignatureNotificationsApi & SlotNotificationsApi & BlockNotificationsApi & SlotsUpdatesNotificationsApi & VoteNotificationsApi>;
//# sourceMappingURL=index.d.ts.map