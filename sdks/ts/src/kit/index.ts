import { address, type ClientWithIdentity, createClient, createNoopSigner, createSolanaRpc, pipe } from '@solana/kit';
import {
    planAndSendTransactions,
    transactionPlanExecutor as transactionPlanExecutorPlugin,
    transactionPlanner as transactionPlannerPlugin,
} from '@solana/kit-plugin-instruction-plan';
import { solanaRpcConnection } from '@solana/kit-plugin-rpc';
import { identity, payer } from '@solana/kit-plugin-signer';
import {
    estimateAndUpdateProvisoryComputeUnitLimitFactory,
    estimateComputeUnitLimitFactory,
} from '@solana-program/compute-budget';

import { KoraClient } from '../client.js';
import { koraPlugin } from '../plugin.js';
import type { KoraBundleConfig, KoraKitClientConfig } from '../types/index.js';
import { createKoraTransactionPlanExecutor } from './executor.js';
import { buildPlaceholderPaymentInstruction, koraPaymentAddress } from './payment.js';
import { buildComputeBudgetInstructions, createKoraTransactionPlanner } from './planner.js';

/**
 * Kora bundle plugin.
 *
 * Composes RPC + payer + Kora RPC methods + transaction planning/execution into a single
 * plugin that mirrors the shape of `solanaRpc` from `@solana/kit-plugin-rpc`.
 *
 * The caller's signer is read from `client.identity` — install `identity()` (or `signer()`)
 * from `@solana/kit-plugin-signer` before this plugin. The fee payer (Kora's address) is
 * fetched from the Kora node and installed internally as a noop signer, overriding any
 * `payer` already on the client.
 *
 * @beta This API is experimental and may change in future releases.
 *
 * @example
 * ```ts
 * import { createClient } from '@solana/kit';
 * import { identity } from '@solana/kit-plugin-signer';
 * import { kora } from '@solana/kora';
 *
 * const client = await createClient()
 *   .use(identity(userSigner))
 *   .use(kora({
 *     endpoint: 'https://kora.example.com',
 *     rpcUrl: 'https://api.mainnet-beta.solana.com',
 *     feeToken: address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
 *   }));
 *
 * const signature = await client.sendTransaction([myInstruction]);
 * ```
 */
export function kora(config: KoraBundleConfig) {
    return async <T extends ClientWithIdentity>(client: T) => {
        const koraClient = new KoraClient({
            apiKey: config.apiKey,
            getRecaptchaToken: config.getRecaptchaToken,
            hmacSecret: config.hmacSecret,
            rpcUrl: config.endpoint,
        });

        const { signer_address, payment_address } = await koraClient.getPayerSigner();
        const paymentAddr = payment_address ? address(payment_address) : undefined;
        const payerSigner = createNoopSigner(address(signer_address));

        const computeBudgetIxs = buildComputeBudgetInstructions(config);
        const solanaRpc = createSolanaRpc(config.rpcUrl);

        const hasCuEstimation = config.computeUnitLimit === undefined;
        const resolveProvisoryComputeUnitLimit = hasCuEstimation
            ? estimateAndUpdateProvisoryComputeUnitLimitFactory(estimateComputeUnitLimitFactory({ rpc: solanaRpc }))
            : undefined;

        const userSigner = client.identity;

        const payment = paymentAddr
            ? await buildPlaceholderPaymentInstruction(userSigner, paymentAddr, config.feeToken, config.tokenProgramId)
            : undefined;

        const koraTransactionPlanner = createKoraTransactionPlanner(
            payerSigner,
            computeBudgetIxs,
            payment?.instruction,
            hasCuEstimation,
        );

        const koraTransactionPlanExecutor = createKoraTransactionPlanExecutor(
            koraClient,
            config,
            userSigner,
            payerSigner,
            payment
                ? {
                      destinationTokenAccount: payment.destinationTokenAccount,
                      sourceTokenAccount: payment.sourceTokenAccount,
                  }
                : undefined,
            resolveProvisoryComputeUnitLimit,
        );

        return pipe(
            client,
            payer(payerSigner),
            solanaRpcConnection(config.rpcUrl),
            koraPlugin({
                endpoint: config.endpoint,
                koraClient,
            }),
            koraPaymentAddress(paymentAddr),
            transactionPlannerPlugin(koraTransactionPlanner),
            transactionPlanExecutorPlugin(koraTransactionPlanExecutor),
            planAndSendTransactions(),
        );
    };
}

/**
 * The type returned by {@link createKitKoraClient}.
 *
 * @deprecated Derive the client type from your own `createClient().use(...).use(kora(...))`
 * composition instead. Will be removed in 0.4.0.
 */
export type KoraKitClient = Awaited<ReturnType<typeof createKitKoraClient>>;

/**
 * Creates a Kora Kit client composed from Kit plugins.
 *
 * @beta This API is experimental and may change in future releases.
 *
 * @deprecated Compose with the {@link kora} plugin instead:
 * ```ts
 * import { createClient } from '@solana/kit';
 * import { identity } from '@solana/kit-plugin-signer';
 * import { kora } from '@solana/kora';
 *
 * const client = await createClient()
 *   .use(identity(userSigner))
 *   .use(kora({ endpoint, rpcUrl, feeToken }));
 * ```
 * Will be removed in 0.4.0.
 */
// TODO: Bundle support — the plan/execute pipeline currently handles single transactions only.
// For Jito bundles, users must manually encode transactions and call `client.kora.signAndSendBundle()`.
export async function createKitKoraClient(config: KoraKitClientConfig) {
    const { feePayerWallet, ...bundleConfig } = config;
    return await createClient().use(identity(feePayerWallet)).use(kora(bundleConfig));
}
