import type { Address } from '@solana/addresses';
import { SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM, SolanaError } from '@solana/errors';
export declare function isProgramError<TProgramErrorCode extends number>(error: unknown, transactionMessage: {
    instructions: Record<number, {
        programAddress: Address;
    }>;
}, programAddress: Address, code?: TProgramErrorCode): error is Readonly<{
    context: Readonly<{
        code: TProgramErrorCode;
    }>;
}> & SolanaError<typeof SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM>;
//# sourceMappingURL=program-error.d.ts.map