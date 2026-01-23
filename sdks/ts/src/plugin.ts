import { address, blockhash, type Address, type Blockhash, type Base64EncodedWireTransaction } from '@solana/kit';
import { KoraClient } from './client.js';
import type {
    KoraPluginConfig,
    KitPayerSignerResponse,
    KitBlockhashResponse,
    KitSupportedTokensResponse,
    KitEstimateFeeResponse,
    KitSignTransactionResponse,
    KitSignAndSendTransactionResponse,
    KitTransferTransactionResponse,
    KitPaymentInstructionResponse,
    KitConfigResponse,
    EstimateTransactionFeeRequest,
    SignTransactionRequest,
    SignAndSendTransactionRequest,
    TransferTransactionRequest,
    GetPaymentInstructionRequest,
} from './types/index.js';

/**
 * Creates a Kora Kit plugin that adds Kora paymaster functionality to a Kit client.
 *
 * The plugin exposes all Kora RPC methods with Kit-typed responses (Address, Blockhash).
 *
 * **Note:** The plugin pattern with `createEmptyClient().use()` requires `@solana/kit` v5.4.0+.
 * For older kit versions, use `KoraClient` directly instead.
 *
 * @param config - Plugin configuration
 * @param config.endpoint - Kora RPC endpoint URL
 * @param config.apiKey - Optional API key for authentication
 * @param config.hmacSecret - Optional HMAC secret for signature-based authentication
 * @returns A Kit plugin function that adds `.kora` to the client
 *
 * @example
 * ```typescript
 * import { createEmptyClient } from '@solana/kit';
 * import { koraPlugin } from '@solana/kora';
 *
 * const client = createEmptyClient()
 *   .use(koraPlugin({ endpoint: 'https://kora.example.com' }));
 *
 * // All responses have Kit-typed fields
 * const config = await client.kora.getConfig();
 * // config.fee_payers is Address[] not string[]
 *
 * const { signer_pubkey } = await client.kora.signTransaction({ transaction: tx });
 * // signer_pubkey is Address not string
 * ```
 */
export function koraPlugin(config: KoraPluginConfig) {
    const client = new KoraClient({
        rpcUrl: config.endpoint,
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
    });

    return <T extends object>(c: T) => ({
        ...c,
        kora: {
            /**
             * Retrieves the current Kora server configuration with Kit-typed addresses.
             */
            async getConfig(): Promise<KitConfigResponse> {
                const result = await client.getConfig();
                return {
                    fee_payers: result.fee_payers.map(addr => address(addr)),
                    validation_config: {
                        ...result.validation_config,
                        allowed_programs: result.validation_config.allowed_programs.map(addr => address(addr)),
                        allowed_tokens: result.validation_config.allowed_tokens.map(addr => address(addr)),
                        allowed_spl_paid_tokens: result.validation_config.allowed_spl_paid_tokens.map(addr =>
                            address(addr),
                        ),
                        disallowed_accounts: result.validation_config.disallowed_accounts.map(addr => address(addr)),
                    },
                    enabled_methods: result.enabled_methods,
                };
            },

            /**
             * Retrieves the payer signer and payment destination with Kit-typed addresses.
             */
            async getPayerSigner(): Promise<KitPayerSignerResponse> {
                const result = await client.getPayerSigner();
                return {
                    signer_address: address(result.signer_address),
                    payment_address: address(result.payment_address),
                };
            },

            /**
             * Gets the latest blockhash with Kit Blockhash type.
             */
            async getBlockhash(): Promise<KitBlockhashResponse> {
                const result = await client.getBlockhash();
                return {
                    blockhash: blockhash(result.blockhash),
                };
            },

            /**
             * Retrieves the list of tokens supported for fee payment with Kit-typed addresses.
             */
            async getSupportedTokens(): Promise<KitSupportedTokensResponse> {
                const result = await client.getSupportedTokens();
                return {
                    tokens: result.tokens.map(addr => address(addr)),
                };
            },

            /**
             * Estimates the transaction fee with Kit-typed addresses.
             */
            async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<KitEstimateFeeResponse> {
                const result = await client.estimateTransactionFee(request);
                return {
                    fee_in_lamports: result.fee_in_lamports,
                    fee_in_token: result.fee_in_token,
                    signer_pubkey: address(result.signer_pubkey),
                    payment_address: address(result.payment_address),
                };
            },

            /**
             * Signs a transaction with Kit-typed response.
             */
            async signTransaction(request: SignTransactionRequest): Promise<KitSignTransactionResponse> {
                const result = await client.signTransaction(request);
                return {
                    signed_transaction: result.signed_transaction as Base64EncodedWireTransaction,
                    signer_pubkey: address(result.signer_pubkey),
                };
            },

            /**
             * Signs and sends a transaction with Kit-typed response.
             */
            async signAndSendTransaction(
                request: SignAndSendTransactionRequest,
            ): Promise<KitSignAndSendTransactionResponse> {
                const result = await client.signAndSendTransaction(request);
                return {
                    signed_transaction: result.signed_transaction as Base64EncodedWireTransaction,
                    signer_pubkey: address(result.signer_pubkey),
                };
            },

            /**
             * Creates a token transfer transaction with Kit-typed response.
             */
            async transferTransaction(request: TransferTransactionRequest): Promise<KitTransferTransactionResponse> {
                const result = await client.transferTransaction(request);
                return {
                    transaction: result.transaction as Base64EncodedWireTransaction,
                    message: result.message,
                    blockhash: blockhash(result.blockhash),
                    signer_pubkey: address(result.signer_pubkey),
                    instructions: result.instructions,
                };
            },

            /**
             * Creates a payment instruction with Kit-typed response.
             */
            async getPaymentInstruction(request: GetPaymentInstructionRequest): Promise<KitPaymentInstructionResponse> {
                const result = await client.getPaymentInstruction(request);
                return {
                    original_transaction: result.original_transaction as Base64EncodedWireTransaction,
                    payment_instruction: result.payment_instruction,
                    payment_amount: result.payment_amount,
                    payment_token: address(result.payment_token),
                    payment_address: address(result.payment_address),
                    signer_address: address(result.signer_address),
                };
            },
        },
    });
}

/** Type representing the Kora API exposed by the plugin */
export type KoraApi = ReturnType<ReturnType<typeof koraPlugin>>['kora'];
