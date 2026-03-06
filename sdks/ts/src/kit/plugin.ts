import { address, type Base64EncodedWireTransaction, blockhash, signature } from '@solana/kit';

import { KoraClient } from '../client.js';
import type {
    EstimateTransactionFeeRequest,
    GetPaymentInstructionRequest,
    KitBlockhashResponse,
    KitConfigResponse,
    KitEstimateFeeResponse,
    KitPayerSignerResponse,
    KitPaymentInstructionResponse,
    KitSignAndSendTransactionResponse,
    KitSignTransactionResponse,
    KitSupportedTokensResponse,
    KitTransferTransactionResponse,
    KoraPluginConfig,
    SignAndSendTransactionRequest,
    SignTransactionRequest,
    TransferTransactionRequest,
} from '../types/index.js';

/**
 * Kit plugin that adds `.kora` namespace with all Kora RPC methods.
 * Responses are cast to Kit types (Address, Blockhash, Signature).
 *
 * Requires `@solana/kit` v5.4.0+ for the `createEmptyClient().use()` pattern.
 *
 * @example
 * ```typescript
 * const client = createEmptyClient()
 *   .use(koraPlugin({ endpoint: 'https://kora.example.com' }));
 *
 * const config = await client.kora.getConfig();
 * const { signer_pubkey } = await client.kora.signTransaction({ transaction: tx });
 * ```
 */
export function koraPlugin(config: KoraPluginConfig) {
    const client = new KoraClient({
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
        rpcUrl: config.endpoint,
    });

    return <T extends object>(c: T) => ({
        ...c,
        kora: {
            async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<KitEstimateFeeResponse> {
                const result = await client.estimateTransactionFee(request);
                return {
                    fee_in_lamports: result.fee_in_lamports,
                    fee_in_token: result.fee_in_token,
                    payment_address: address(result.payment_address),
                    signer_pubkey: address(result.signer_pubkey),
                };
            },

            async getBlockhash(): Promise<KitBlockhashResponse> {
                const result = await client.getBlockhash();
                return {
                    blockhash: blockhash(result.blockhash),
                };
            },

            async getConfig(): Promise<KitConfigResponse> {
                const result = await client.getConfig();
                return {
                    enabled_methods: result.enabled_methods,
                    fee_payers: result.fee_payers.map(addr => address(addr)),
                    validation_config: {
                        ...result.validation_config,
                        allowed_programs: result.validation_config.allowed_programs.map(addr => address(addr)),
                        allowed_spl_paid_tokens: result.validation_config.allowed_spl_paid_tokens.map(addr =>
                            address(addr),
                        ),
                        allowed_tokens: result.validation_config.allowed_tokens.map(addr => address(addr)),
                        disallowed_accounts: result.validation_config.disallowed_accounts.map(addr => address(addr)),
                    },
                };
            },

            async getPayerSigner(): Promise<KitPayerSignerResponse> {
                const result = await client.getPayerSigner();
                return {
                    payment_address: address(result.payment_address),
                    signer_address: address(result.signer_address),
                };
            },

            async getPaymentInstruction(request: GetPaymentInstructionRequest): Promise<KitPaymentInstructionResponse> {
                const result = await client.getPaymentInstruction(request);
                return {
                    original_transaction: result.original_transaction as Base64EncodedWireTransaction,
                    payment_address: address(result.payment_address),
                    payment_amount: result.payment_amount,
                    payment_instruction: result.payment_instruction,
                    payment_token: address(result.payment_token),
                    signer_address: address(result.signer_address),
                };
            },

            async getSupportedTokens(): Promise<KitSupportedTokensResponse> {
                const result = await client.getSupportedTokens();
                return {
                    tokens: result.tokens.map(addr => address(addr)),
                };
            },

            async signAndSendTransaction(
                request: SignAndSendTransactionRequest,
            ): Promise<KitSignAndSendTransactionResponse> {
                const result = await client.signAndSendTransaction(request);
                return {
                    signature: signature(result.signature),
                    signed_transaction: result.signed_transaction as Base64EncodedWireTransaction,
                    signer_pubkey: address(result.signer_pubkey),
                };
            },

            async signTransaction(request: SignTransactionRequest): Promise<KitSignTransactionResponse> {
                const result = await client.signTransaction(request);
                return {
                    signed_transaction: result.signed_transaction as Base64EncodedWireTransaction,
                    signer_pubkey: address(result.signer_pubkey),
                };
            },

            async transferTransaction(request: TransferTransactionRequest): Promise<KitTransferTransactionResponse> {
                const result = await client.transferTransaction(request);
                return {
                    blockhash: blockhash(result.blockhash),
                    instructions: result.instructions,
                    message: result.message,
                    signer_pubkey: address(result.signer_pubkey),
                    transaction: result.transaction as Base64EncodedWireTransaction,
                };
            },
        },
    });
}

/** Type representing the Kora plugin namespace on the client. */
export type KoraPlugin = ReturnType<ReturnType<typeof koraPlugin>>['kora'];
