import { address, createClient, createNoopSigner, createSolanaRpc } from '@solana/kit';
import {
    planAndSendTransactions,
    transactionPlanExecutor as transactionPlanExecutorPlugin,
    transactionPlanner as transactionPlannerPlugin,
} from '@solana/kit-plugin-instruction-plan';
import { solanaRpcConnection } from '@solana/kit-plugin-rpc';
import { payer } from '@solana/kit-plugin-signer';
import {
    estimateAndUpdateProvisoryComputeUnitLimitFactory,
    estimateComputeUnitLimitFactory,
} from '@solana-program/compute-budget';

import { KoraClient } from '../client.js';
import { koraPlugin } from '../plugin.js';
import type { KoraKitClientConfig } from '../types/index.js';
import { createKoraTransactionPlanExecutor } from './executor.js';
import { buildPlaceholderPaymentInstruction, koraPaymentAddress } from './payment.js';
import { buildComputeBudgetInstructions, createKoraTransactionPlanner } from './planner.js';

/** The type returned by {@link createKitKoraClient}. */
export type KoraKitClient = Awaited<ReturnType<typeof createKitKoraClient>>;

/**
 * Creates a Kora Kit client composed from Kit plugins.
 *
 * The returned client satisfies `ClientWithPayer`, `ClientWithTransactionPlanning`,
 * and `ClientWithTransactionSending`, making it composable with Kit program plugins.
 *
 * @beta This API is experimental and may change in future releases.
 *
 * @example
 * ```typescript
 * import { createKitKoraClient } from '@solana/kora';
 * import { address } from '@solana/kit';
 *
 * const client = await createKitKoraClient({
 *   endpoint: 'https://kora.example.com',
 *   rpcUrl: 'https://api.mainnet-beta.solana.com',
 *   feeToken: address('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
 *   feePayerWallet: userSigner,
 * });
 *
 * const result = await client.sendTransaction([myInstruction]);
 * ```
 */
// TODO: Bundle support — the plan/execute pipeline currently handles single transactions only.
// For Jito bundles, users must manually encode transactions and call `client.kora.signAndSendBundle()`.
// A future `createKitKoraBundleClient` (or a bundle-aware executor plugin) could extend this to
// plan multiple transaction messages and submit them as a single bundle.
export async function createKitKoraClient(config: KoraKitClientConfig) {
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

    const payment = paymentAddr
        ? await buildPlaceholderPaymentInstruction(
              config.feePayerWallet,
              paymentAddr,
              config.feeToken,
              config.tokenProgramId,
          )
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
        payerSigner,
        payment
            ? {
                  destinationTokenAccount: payment.destinationTokenAccount,
                  sourceTokenAccount: payment.sourceTokenAccount,
              }
            : undefined,
        resolveProvisoryComputeUnitLimit,
    );

    return createClient()
        .use(solanaRpcConnection(config.rpcUrl))
        .use(
            koraPlugin({
                endpoint: config.endpoint,
                koraClient,
            }),
        )
        .use(payer(payerSigner))
        .use(koraPaymentAddress(paymentAddr))
        .use(transactionPlannerPlugin(koraTransactionPlanner))
        .use(transactionPlanExecutorPlugin(koraTransactionPlanExecutor))
        .use(planAndSendTransactions());
}
