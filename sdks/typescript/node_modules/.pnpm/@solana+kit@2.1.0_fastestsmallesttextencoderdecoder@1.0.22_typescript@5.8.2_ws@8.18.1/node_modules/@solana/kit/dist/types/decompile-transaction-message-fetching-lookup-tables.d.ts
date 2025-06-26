import { type FetchAccountsConfig } from '@solana/accounts';
import type { GetMultipleAccountsApi, Rpc } from '@solana/rpc';
import { CompilableTransactionMessage, CompiledTransactionMessage } from '@solana/transaction-messages';
type DecompileTransactionMessageFetchingLookupTablesConfig = FetchAccountsConfig & {
    lastValidBlockHeight?: bigint;
};
export declare function decompileTransactionMessageFetchingLookupTables(compiledTransactionMessage: CompiledTransactionMessage, rpc: Rpc<GetMultipleAccountsApi>, config?: DecompileTransactionMessageFetchingLookupTablesConfig): Promise<CompilableTransactionMessage>;
export {};
//# sourceMappingURL=decompile-transaction-message-fetching-lookup-tables.d.ts.map