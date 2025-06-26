import { SOLANA_ERROR__RPC__INTEGER_OVERFLOW, SolanaError } from '@solana/errors';
import type { KeyPath } from '@solana/rpc-transformers';
export declare function createSolanaJsonRpcIntegerOverflowError(methodName: string, keyPath: KeyPath, value: bigint): SolanaError<typeof SOLANA_ERROR__RPC__INTEGER_OVERFLOW>;
//# sourceMappingURL=rpc-integer-overflow-error.d.ts.map