import { VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
import { TransactionVersion } from '../transaction-message';
export declare function getTransactionVersionEncoder(): VariableSizeEncoder<TransactionVersion>;
export declare function getTransactionVersionDecoder(): VariableSizeDecoder<TransactionVersion>;
export declare function getTransactionVersionCodec(): VariableSizeCodec<TransactionVersion>;
//# sourceMappingURL=transaction-version.d.ts.map