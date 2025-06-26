import { Rpc, SimulateTransactionApi } from '@solana/rpc';
import { CompilableTransactionMessage, ITransactionMessageWithFeePayer, TransactionMessage } from '@solana/transaction-messages';
import { getComputeUnitEstimateForTransactionMessage_INTERNAL_ONLY_DO_NOT_EXPORT } from './compute-limit-internal';
type ComputeUnitEstimateForTransactionMessageFactoryConfig = Readonly<{
    rpc: Rpc<SimulateTransactionApi>;
}>;
type ComputeUnitEstimateForTransactionMessageFunction = (transactionMessage: CompilableTransactionMessage | (ITransactionMessageWithFeePayer & TransactionMessage), config?: Omit<Parameters<typeof getComputeUnitEstimateForTransactionMessage_INTERNAL_ONLY_DO_NOT_EXPORT>[0], 'rpc' | 'transactionMessage'>) => Promise<number>;
export declare function getComputeUnitEstimateForTransactionMessageFactory({ rpc, }: ComputeUnitEstimateForTransactionMessageFactoryConfig): ComputeUnitEstimateForTransactionMessageFunction;
export {};
//# sourceMappingURL=compute-limit.d.ts.map