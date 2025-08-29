import { Instruction, Address } from '@solana/kit';

/**
 * Request Types
 */

/**
 * Parameters for creating a token transfer transaction.
 */
export interface TransferTransactionRequest {
    /** Amount to transfer in the token's smallest unit (e.g., lamports for SOL) */
    amount: number;
    /** Mint address of the token to transfer */
    token: string;
    /** Public key of the source wallet (not token account) */
    source: string;
    /** Public key of the destination wallet (not token account) */
    destination: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
}

/**
 * Parameters for signing a transaction.
 */
export interface SignTransactionRequest {
    /** Base64-encoded transaction to sign */
    transaction: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Parameters for signing and sending a transaction.
 */
export interface SignAndSendTransactionRequest {
    /** Base64-encoded transaction to sign and send */
    transaction: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Parameters for conditionally signing a transaction based on payment.
 */
export interface SignTransactionIfPaidRequest {
    /** Base64-encoded transaction */
    transaction: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Parameters for estimating transaction fees.
 */
export interface EstimateTransactionFeeRequest {
    /** Base64-encoded transaction to estimate fees for */
    transaction: string;
    /** Mint address of the token to calculate fees in */
    fee_token: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Parameters for getting a payment instruction.
 */
export interface GetPaymentInstructionRequest {
    /** Base64-encoded transaction to estimate fees for */
    transaction: string;
    /** Mint address of the token to calculate fees in */
    fee_token: string;
    /** The wallet owner (not token account) that will be making the token payment */
    source_wallet: string;
    /** The token program id to use for the payment (defaults to TOKEN_PROGRAM_ID) */
    token_program_id?: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Response Types
 */

/**
 * Response from creating a transfer transaction.
 */
export interface TransferTransactionResponse {
    /** Base64-encoded signed transaction */
    transaction: string;
    /** Base64-encoded message */
    message: string;
    /** Recent blockhash used in the transaction */
    blockhash: string;
    /** Public key of the signer used to send the transaction */
    signer_pubkey: string;
    /** Parsed instructions from the transaction message */
    instructions: Instruction[];
}

/**
 * Response from signing a transaction.
 */
export interface SignTransactionResponse {
    /** Base58-encoded signature */
    signature: string;
    /** Base64-encoded signed transaction */
    signed_transaction: string;
    /** Public key of the signer used to sign the transaction */
    signer_pubkey: string;
}

/**
 * Response from signing and sending a transaction.
 */
export interface SignAndSendTransactionResponse {
    /** Base58-encoded transaction signature */
    signature: string;
    /** Base64-encoded signed transaction */
    signed_transaction: string;
    /** Public key of the signer used to send the transaction */
    signer_pubkey: string;
}

/**
 * Response from conditionally signing a transaction.
 */
export interface SignTransactionIfPaidResponse {
    /** Base64-encoded original transaction */
    transaction: string;
    /** Base64-encoded signed transaction */
    signed_transaction: string;
    /** Public key of the signer used to sign the transaction */
    signer_pubkey: string;
}

/**
 * Response containing the latest blockhash.
 */
export interface GetBlockhashResponse {
    /** Base58-encoded blockhash */
    blockhash: string;
}

/**
 * Response containing supported token mint addresses.
 */
export interface GetSupportedTokensResponse {
    /** Array of supported token mint addresses */
    tokens: string[];
}

/**
 * Response containing estimated transaction fees.
 */
export interface EstimateTransactionFeeResponse {
    /** Transaction fee in lamports */
    fee_in_lamports: number;
    /**
     * Transaction fee in the requested token (in decimals value of the token, e.g. 10^6 for USDC)
     */
    fee_in_token: number;
    /** Public key of the signer used to estimate the fee */
    signer_pubkey: string;
    /** Public key of the payment destination */
    payment_address: string;
}

/**
 * Response containing the payer signer and payment destination.
 */
export interface GetPayerSignerResponse {
    /** Public key of the payer signer */
    signer_address: string;
    /** Public key of the payment destination */
    payment_address: string;
}

/**
 * Response containing a payment instruction.
 */
export interface GetPaymentInstructionResponse {
    /** Base64-encoded original transaction */
    original_transaction: string;
    /** Base64-encoded payment instruction */
    payment_instruction: Instruction;
    /** Payment amount in the requested token */
    payment_amount: number;
    /** Mint address of the token used for payment */
    payment_token: string;
    /** Public key of the payment destination */
    payment_address: string;
    /** Public key of the payer signer */
    signer_address: string;
}

/**
 * Configuration Types
 */

export type PriceSource = 'Jupiter' | 'Mock';

/**
 * Validation configuration for the Kora server.
 */
export interface ValidationConfig {
    /** Maximum allowed transaction value in lamports */
    max_allowed_lamports: number;
    /** Maximum number of signatures allowed per transaction */
    max_signatures: number;
    /** Price oracle source for token conversions */
    price_source: PriceSource;
    /** List of allowed Solana program IDs */
    allowed_programs: string[];
    /** List of allowed token mint addresses for fee payment */
    allowed_tokens: string[];
    /** List of SPL tokens accepted for paid transactions */
    allowed_spl_paid_tokens: string[];
    /** List of blocked account addresses */
    disallowed_accounts: string[];
    /** Policy controlling fee payer permissions */
    fee_payer_policy: FeePayerPolicy;
    /** Pricing model configuration */
    price: PriceConfig;
    /** Token2022 configuration */
    token2022: Token2022Config;
}

/**
 * Blocked extensions for Token2022.
 */
export interface Token2022Config {
    /** List of blocked mint extensions */
    blocked_mint_extensions: string[];
    /** List of blocked account extensions */
    blocked_account_extensions: string[];
}

/**
 * Pricing model for transaction fees.
 * @remarks
 * - `margin`: Adds a percentage margin to base fees
 * - `fixed`: Charges a fixed amount in a specific token
 * - `free`: No additional fees charged
 */
export type PriceModel =
    | { type: 'margin'; margin: number }
    | { type: 'fixed'; amount: number; token: string }
    | { type: 'free' };

export type PriceConfig = PriceModel;

/**
 * Enabled status for methods for the Kora server.
 */
export interface EnabledMethods {
    /** Whether the liveness method is enabled */
    liveness: boolean;
    /** Whether the estimate_transaction_fee method is enabled */
    estimate_transaction_fee: boolean;
    /** Whether the get_supported_tokens method is enabled */
    get_supported_tokens: boolean;
    /** Whether the sign_transaction method is enabled */
    sign_transaction: boolean;
    /** Whether the sign_and_send_transaction method is enabled */
    sign_and_send_transaction: boolean;
    /** Whether the transfer_transaction method is enabled */
    transfer_transaction: boolean;
    /** Whether the get_blockhash method is enabled */
    get_blockhash: boolean;
    /** Whether the get_config method is enabled */
    get_config: boolean;
    /** Whether the sign_transaction_if_paid method is enabled */
    sign_transaction_if_paid: boolean;
}

/**
 * Kora server configuration.
 */
export interface Config {
    /** Array of public keys of the fee payer accounts (signer pool) */
    fee_payers: string[];
    /** Validation rules and constraints */
    validation_config: ValidationConfig;
    /** Enabled methods */
    enabled_methods: EnabledMethods;
}

/**
 * Policy controlling what actions the fee payer can perform.
 */
export interface FeePayerPolicy {
    /** Allow fee payer to be source in SOL transfers */
    allow_sol_transfers: boolean;
    /** Allow fee payer to be source in SPL token transfers */
    allow_spl_transfers: boolean;
    /** Allow fee payer to be source in Token2022 transfers */
    allow_token2022_transfers: boolean;
    /** Allow fee payer to use Assign instruction */
    allow_assign: boolean;
    /** Allow fee payer to use Burn instruction */
    allow_burn: boolean;
    /** Allow fee payer to use CloseAccount instruction */
    allow_close_account: boolean;
    /** Allow fee payer to use Approve instruction */
    allow_approve: boolean;
}

/**
 * RPC Types
 */

/**
 * JSON-RPC error object.
 */
export interface RpcError {
    /** Error code */
    code: number;
    /** Human-readable error message */
    message: string;
}

/**
 * JSON-RPC request structure.
 * @typeParam T - Type of the params object
 */
export interface RpcRequest<T> {
    /** JSON-RPC version */
    jsonrpc: '2.0';
    /** Request ID */
    id: number;
    /** RPC method name */
    method: string;
    /** Method parameters */
    params: T;
}
/**
 * Authentication headers for API requests.
 */
export interface AuthenticationHeaders {
    /** API key for simple authentication */
    'x-api-key'?: string;
    /** Unix timestamp for HMAC authentication */
    'x-timestamp'?: string;
    /** HMAC SHA256 signature of timestamp + body */
    'x-hmac-signature'?: string;
}

/**
 * Options for initializing a Kora client.
 */
export interface KoraClientOptions {
    /** URL of the Kora RPC server */
    rpcUrl: string;
    /** Optional API key for authentication */
    apiKey?: string;
    /** Optional HMAC secret for signature-based authentication */
    hmacSecret?: string;
}
