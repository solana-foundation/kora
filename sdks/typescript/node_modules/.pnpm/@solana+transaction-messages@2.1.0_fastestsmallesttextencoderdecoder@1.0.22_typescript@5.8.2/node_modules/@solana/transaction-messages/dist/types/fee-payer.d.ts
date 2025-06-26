import { Address } from '@solana/addresses';
import { BaseTransactionMessage } from './transaction-message';
export interface ITransactionMessageWithFeePayer<TAddress extends string = string> {
    readonly feePayer: Readonly<{
        address: Address<TAddress>;
    }>;
}
export declare function setTransactionMessageFeePayer<TFeePayerAddress extends string, TTransactionMessage extends BaseTransactionMessage & Partial<ITransactionMessageWithFeePayer>>(feePayer: Address<TFeePayerAddress>, transactionMessage: TTransactionMessage): ITransactionMessageWithFeePayer<TFeePayerAddress> & Omit<TTransactionMessage, 'feePayer'>;
//# sourceMappingURL=fee-payer.d.ts.map