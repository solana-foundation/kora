import {
    address,
    blockhash,
    signature,
    type Address,
    type Blockhash,
    type Base64EncodedWireTransaction,
} from '@solana/kit';
import { KoraClient } from '../client.js';
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
        rpcUrl: config.endpoint,
        apiKey: config.apiKey,
        hmacSecret: config.hmacSecret,
    });

    return <T extends object>(c: T) => ({
        ...c,
        kora: {
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

            async getPayerSigner(): Promise<KitPayerSignerResponse> {
                const result = await client.getPayerSigner();
                return {
                    signer_address: address(result.signer_address),
                    payment_address: address(result.payment_address),
                };
            },

            async getBlockhash(): Promise<KitBlockhashResponse> {
                const result = await client.getBlockhash();
                return {
                    blockhash: blockhash(result.blockhash),
                };
            },

            async getSupportedTokens(): Promise<KitSupportedTokensResponse> {
                const result = await client.getSupportedTokens();
                return {
                    tokens: result.tokens.map(addr => address(addr)),
                };
            },

            async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<KitEstimateFeeResponse> {
                const result = await client.estimateTransactionFee(request);
                return {
                    fee_in_lamports: result.fee_in_lamports,
                    fee_in_token: result.fee_in_token,
                    signer_pubkey: address(result.signer_pubkey),
                    payment_address: address(result.payment_address),
                };
            },

            async signTransaction(request: SignTransactionRequest): Promise<KitSignTransactionResponse> {
                const result = await client.signTransaction(request);
                return {
                    signed_transaction: result.signed_transaction as Base64EncodedWireTransaction,
                    signer_pubkey: address(result.signer_pubkey),
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

/** Type representing the Kora plugin namespace on the client. */
export type KoraPlugin = ReturnType<ReturnType<typeof koraPlugin>>['kora'];
