import type { Address } from '@solana/addresses';
import type { Signature } from '@solana/keys';
import type { RequestAirdropApi, Rpc } from '@solana/rpc';
import type { Commitment, Lamports } from '@solana/rpc-types';
import { waitForRecentTransactionConfirmationUntilTimeout } from '@solana/transaction-confirmation';
type RequestAndConfirmAirdropConfig = Readonly<{
    abortSignal?: AbortSignal;
    commitment: Commitment;
    confirmSignatureOnlyTransaction: (config: Omit<Parameters<typeof waitForRecentTransactionConfirmationUntilTimeout>[0], 'getRecentSignatureConfirmationPromise' | 'getTimeoutPromise'>) => Promise<void>;
    lamports: Lamports;
    recipientAddress: Address;
    rpc: Rpc<RequestAirdropApi>;
}>;
export declare function requestAndConfirmAirdrop_INTERNAL_ONLY_DO_NOT_EXPORT({ abortSignal, commitment, confirmSignatureOnlyTransaction, lamports, recipientAddress, rpc, }: RequestAndConfirmAirdropConfig): Promise<Signature>;
export {};
//# sourceMappingURL=airdrop-internal.d.ts.map