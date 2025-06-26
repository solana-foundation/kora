import type { Address } from '@solana/addresses';
import type { Signature } from '@solana/keys';
import type { Commitment, SolanaRpcResponse, TransactionError } from '@solana/rpc-types';
type LogsNotificationsApiNotification = SolanaRpcResponse<Readonly<{
    err: TransactionError | null;
    logs: readonly string[] | null;
    signature: Signature;
}>>;
type LogsNotificationsApiFilter = 'all' | 'allWithVotes' | {
    mentions: [Address];
};
type LogsNotificationsApiConfig = Readonly<{
    commitment?: Commitment;
}>;
export type LogsNotificationsApi = {
    /**
     * Subscribe to a transaction logs to receive notification when a given transaction is committed.
     * On `logsNotification` - the subscription is automatically cancelled.
     * The logs must be a txid, the first logs of a transaction.
     */
    logsNotifications(filter: LogsNotificationsApiFilter, config?: LogsNotificationsApiConfig): LogsNotificationsApiNotification;
};
export {};
//# sourceMappingURL=logs-notifications.d.ts.map