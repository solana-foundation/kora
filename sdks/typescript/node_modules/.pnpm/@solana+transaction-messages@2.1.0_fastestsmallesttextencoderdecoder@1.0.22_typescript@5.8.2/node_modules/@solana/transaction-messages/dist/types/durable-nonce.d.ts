import { Address } from '@solana/addresses';
import { IInstruction, IInstructionWithAccounts, IInstructionWithData, ReadonlyAccount, ReadonlySignerAccount, WritableAccount, WritableSignerAccount } from '@solana/instructions';
import { BaseTransactionMessage } from './transaction-message';
type AdvanceNonceAccountInstruction<TNonceAccountAddress extends string = string, TNonceAuthorityAddress extends string = string> = IInstruction<'11111111111111111111111111111111'> & IInstructionWithAccounts<readonly [
    WritableAccount<TNonceAccountAddress>,
    ReadonlyAccount<'SysvarRecentB1ockHashes11111111111111111111'>,
    ReadonlySignerAccount<TNonceAuthorityAddress> | WritableSignerAccount<TNonceAuthorityAddress>
]> & IInstructionWithData<AdvanceNonceAccountInstructionData>;
type AdvanceNonceAccountInstructionData = Uint8Array & {
    readonly __brand: unique symbol;
};
type DurableNonceConfig<TNonceAccountAddress extends string = string, TNonceAuthorityAddress extends string = string, TNonceValue extends string = string> = Readonly<{
    readonly nonce: Nonce<TNonceValue>;
    readonly nonceAccountAddress: Address<TNonceAccountAddress>;
    readonly nonceAuthorityAddress: Address<TNonceAuthorityAddress>;
}>;
export type Nonce<TNonceValue extends string = string> = TNonceValue & {
    readonly __brand: unique symbol;
};
type NonceLifetimeConstraint<TNonceValue extends string = string> = Readonly<{
    nonce: Nonce<TNonceValue>;
}>;
export interface TransactionMessageWithDurableNonceLifetime<TNonceAccountAddress extends string = string, TNonceAuthorityAddress extends string = string, TNonceValue extends string = string> {
    readonly instructions: readonly [
        AdvanceNonceAccountInstruction<TNonceAccountAddress, TNonceAuthorityAddress>,
        ...IInstruction[]
    ];
    readonly lifetimeConstraint: NonceLifetimeConstraint<TNonceValue>;
}
export declare function assertIsDurableNonceTransactionMessage(transaction: BaseTransactionMessage | (BaseTransactionMessage & TransactionMessageWithDurableNonceLifetime)): asserts transaction is BaseTransactionMessage & TransactionMessageWithDurableNonceLifetime;
export declare function isAdvanceNonceAccountInstruction(instruction: IInstruction): instruction is AdvanceNonceAccountInstruction;
export declare function isDurableNonceTransaction(transaction: BaseTransactionMessage | (BaseTransactionMessage & TransactionMessageWithDurableNonceLifetime)): transaction is BaseTransactionMessage & TransactionMessageWithDurableNonceLifetime;
export declare function setTransactionMessageLifetimeUsingDurableNonce<TTransaction extends BaseTransactionMessage, TNonceAccountAddress extends string = string, TNonceAuthorityAddress extends string = string, TNonceValue extends string = string>({ nonce, nonceAccountAddress, nonceAuthorityAddress, }: DurableNonceConfig<TNonceAccountAddress, TNonceAuthorityAddress, TNonceValue>, transaction: TTransaction | (TransactionMessageWithDurableNonceLifetime & TTransaction)): TransactionMessageWithDurableNonceLifetime<TNonceAccountAddress, TNonceAuthorityAddress, TNonceValue> & TTransaction;
export {};
//# sourceMappingURL=durable-nonce.d.ts.map