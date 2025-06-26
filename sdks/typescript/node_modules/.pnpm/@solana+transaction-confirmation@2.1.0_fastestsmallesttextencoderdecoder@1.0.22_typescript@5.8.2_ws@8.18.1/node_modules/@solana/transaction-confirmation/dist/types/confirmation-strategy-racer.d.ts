import type { Signature } from '@solana/keys';
import type { Commitment } from '@solana/rpc-types';
import { createRecentSignatureConfirmationPromiseFactory } from './confirmation-strategy-recent-signature';
export interface BaseTransactionConfirmationStrategyConfig {
    abortSignal?: AbortSignal;
    commitment: Commitment;
    getRecentSignatureConfirmationPromise: ReturnType<typeof createRecentSignatureConfirmationPromiseFactory>;
}
type WithNonNullableAbortSignal<T> = Omit<T, 'abortSignal'> & Readonly<{
    abortSignal: AbortSignal;
}>;
export declare function raceStrategies<TConfig extends BaseTransactionConfirmationStrategyConfig>(signature: Signature, config: TConfig, getSpecificStrategiesForRace: (config: WithNonNullableAbortSignal<TConfig>) => readonly Promise<unknown>[]): Promise<unknown>;
export {};
//# sourceMappingURL=confirmation-strategy-racer.d.ts.map