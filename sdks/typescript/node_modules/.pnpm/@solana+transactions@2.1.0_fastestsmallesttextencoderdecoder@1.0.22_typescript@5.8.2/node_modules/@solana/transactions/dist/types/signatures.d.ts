import { Signature } from '@solana/keys';
import { Transaction } from './transaction';
export interface FullySignedTransaction extends Transaction {
    readonly __brand: unique symbol;
}
export declare function getSignatureFromTransaction(transaction: Transaction): Signature;
export declare function partiallySignTransaction<T extends Transaction>(keyPairs: CryptoKeyPair[], transaction: T): Promise<T>;
export declare function signTransaction<T extends Transaction>(keyPairs: CryptoKeyPair[], transaction: T): Promise<FullySignedTransaction & T>;
export declare function assertTransactionIsFullySigned(transaction: Transaction): asserts transaction is FullySignedTransaction;
//# sourceMappingURL=signatures.d.ts.map