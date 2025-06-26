import { IAccountLookupMeta, IAccountMeta, IInstruction } from '@solana/instructions';
import { AddressesByLookupTableAddress } from './addresses-by-lookup-table-address';
import { BaseTransactionMessage, TransactionMessage } from './transaction-message';
type TransactionMessageNotLegacy = Exclude<TransactionMessage, {
    version: 'legacy';
}>;
type WidenInstructionAccounts<TInstruction extends IInstruction> = TInstruction extends IInstruction<infer TProgramAddress, infer TAccounts> ? IInstruction<TProgramAddress, {
    [K in keyof TAccounts]: TAccounts[K] extends IAccountMeta<infer TAddress> ? IAccountLookupMeta<TAddress> | IAccountMeta<TAddress> : TAccounts[K];
}> : TInstruction;
type ExtractAdditionalProps<T, U> = Omit<T, keyof U>;
type WidenTransactionMessageInstructions<TTransactionMessage extends TransactionMessage> = TTransactionMessage extends BaseTransactionMessage<infer TVersion, infer TInstruction> ? BaseTransactionMessage<TVersion, WidenInstructionAccounts<TInstruction>> & ExtractAdditionalProps<TTransactionMessage, BaseTransactionMessage<TVersion, WidenInstructionAccounts<TInstruction>>> : TTransactionMessage;
export declare function compressTransactionMessageUsingAddressLookupTables<TTransactionMessage extends TransactionMessageNotLegacy = TransactionMessageNotLegacy>(transactionMessage: TTransactionMessage, addressesByLookupTableAddress: AddressesByLookupTableAddress): TTransactionMessage | WidenTransactionMessageInstructions<TTransactionMessage>;
export {};
//# sourceMappingURL=compress-transaction-message.d.ts.map