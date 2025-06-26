import type { Base58EncodedBytes, Blockhash, Commitment, Reward, Slot, TransactionForAccounts, TransactionForFullBase58, TransactionForFullBase64, TransactionForFullJson, TransactionForFullJsonParsed, UnixTimestamp } from '@solana/rpc-types';
import type { TransactionVersion } from '@solana/transaction-messages';
type GetBlockApiResponseBase = Readonly<{
    /** The number of blocks beneath this block */
    blockHeight: bigint;
    /** Estimated production time, as Unix timestamp */
    blockTime: UnixTimestamp;
    /** the blockhash of this block */
    blockhash: Blockhash;
    /** The slot index of this block's parent */
    parentSlot: Slot;
    /** The blockhash of this block's parent */
    previousBlockhash: Blockhash;
}>;
type GetBlockApiResponseWithRewards = Readonly<{
    /** Block-level rewards */
    rewards: readonly Reward[];
}>;
type GetBlockApiResponseWithSignatures = Readonly<{
    /** List of signatures applied to transactions in this block */
    signatures: readonly Base58EncodedBytes[];
}>;
type GetBlockApiResponseWithTransactions<TTransaction> = Readonly<{
    transactions: readonly TTransaction[];
}>;
type GetBlockCommonConfig = Readonly<{
    /** @defaultValue finalized */
    commitment?: Omit<Commitment, 'processed'>;
}>;
type GetBlockEncoding = 'base58' | 'base64' | 'json' | 'jsonParsed';
type GetBlockMaxSupportedTransactionVersion = Exclude<TransactionVersion, 'legacy'>;
export type GetBlockApi = {
    /**
     * Returns identity and transaction information about a confirmed block in the ledger
     */
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion?: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails: 'none';
    }>): GetBlockApiResponseBase | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion?: GetBlockMaxSupportedTransactionVersion;
        rewards?: true;
        transactionDetails: 'none';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion?: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails: 'signatures';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithSignatures) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion?: GetBlockMaxSupportedTransactionVersion;
        rewards?: true;
        transactionDetails: 'signatures';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithSignatures) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails: 'accounts';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForAccounts<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        rewards: false;
        transactionDetails: 'accounts';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForAccounts<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards?: true;
        transactionDetails: 'accounts';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForAccounts<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: GetBlockEncoding;
        rewards?: true;
        transactionDetails: 'accounts';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForAccounts<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base58';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullBase58<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base58';
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullBase58<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base58';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards?: true;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullBase58<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base58';
        rewards?: true;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullBase58<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base64';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullBase64<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base64';
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullBase64<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base64';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards?: true;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullBase64<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'base64';
        rewards?: true;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullBase64<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'jsonParsed';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullJsonParsed<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'jsonParsed';
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullJsonParsed<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'jsonParsed';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards?: boolean;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullJsonParsed<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding: 'jsonParsed';
        rewards?: boolean;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullJsonParsed<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: 'json';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullJson<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: 'json';
        rewards: false;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithTransactions<TransactionForFullJson<void>>) | null;
    getBlock(slot: Slot, config: GetBlockCommonConfig & Readonly<{
        encoding?: 'json';
        maxSupportedTransactionVersion: GetBlockMaxSupportedTransactionVersion;
        rewards?: boolean;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullJson<GetBlockMaxSupportedTransactionVersion>>) | null;
    getBlock(slot: Slot, config?: GetBlockCommonConfig & Readonly<{
        encoding?: 'json';
        rewards?: boolean;
        transactionDetails?: 'full';
    }>): (GetBlockApiResponseBase & GetBlockApiResponseWithRewards & GetBlockApiResponseWithTransactions<TransactionForFullJson<void>>) | null;
};
export {};
//# sourceMappingURL=getBlock.d.ts.map