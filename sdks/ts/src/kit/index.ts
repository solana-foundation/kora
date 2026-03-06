import { type Address, address, createEmptyClient, createNoopSigner, createSolanaRpc } from '@solana/kit';
import {
    estimateComputeUnitLimitFactory,
    estimateAndUpdateProvisoryComputeUnitLimitFactory,
} from '@solana-program/compute-budget';
import { payer } from '@solana/kit-plugin-payer';
import {
    transactionPlanner as transactionPlannerPlugin,
    transactionPlanExecutor as transactionPlanExecutorPlugin,
    planAndSendTransactions,
} from '@solana/kit-plugin-instruction-plan';
import { rpc } from '@solana/kit-plugin-rpc';
import { KoraClient } from '../client.js';
import { koraPlugin } from './plugin.js';
import type { KoraKitClientConfig } from '../types/index.js';
import { koraPaymentAddress, buildPlaceholderPaymentInstruction } from './payment.js';
import { buildComputeBudgetInstructions, createKoraTransactionPlanner } from './planner.js';
import { createKoraTransactionPlanExecutor } from './executor.js';

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
export async function createKitKoraClient(config: KoraKitClientConfig) {
    const koraClient = new KoraClient({
        rpcUrl: config.endpoint,
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
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
                  sourceTokenAccount: payment.sourceTokenAccount,
                  destinationTokenAccount: payment.destinationTokenAccount,
              }
            : undefined,
        resolveProvisoryComputeUnitLimit,
    );

    return createEmptyClient()
        .use(rpc(config.rpcUrl))
        .use(koraPlugin({ endpoint: config.endpoint, apiKey: config.apiKey, hmacSecret: config.hmacSecret }))
        .use(payer(payerSigner))
        .use(koraPaymentAddress(paymentAddr))
        .use(transactionPlannerPlugin(koraTransactionPlanner))
        .use(transactionPlanExecutorPlugin(koraTransactionPlanExecutor))
        .use(planAndSendTransactions());
}
