import { Instruction } from '@solana/kit';

/**
 * Request Types
 */

/**
 * Parameters for creating a transfer transaction.
 * @deprecated Use `getPaymentInstruction` instead for fee payment flows.
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
    /** Optional signer key to select a specific Kora signer */
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
    /** Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled) */
    user_id?: string;
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
    /** Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled) */
    user_id?: string;
}

/**
 * Parameters for signing a bundle of transactions.
 */
export interface SignBundleRequest {
    /** Array of base64-encoded transactions to sign */
    transactions: string[];
    /** Optional signer address for the transactions */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
    /** Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled) */
    user_id?: string;
    /** Optional indices of transactions to sign (defaults to all if not specified) */
    sign_only_indices?: number[];
}

/**
 * Parameters for signing and sending a bundle of transactions via Jito.
 */
export interface SignAndSendBundleRequest {
    /** Array of base64-encoded transactions to sign and send */
    transactions: string[];
    /** Optional signer address for the transactions */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
    /** Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled) */
    user_id?: string;
    /** Optional indices of transactions to sign (defaults to all if not specified) */
    sign_only_indices?: number[];
}

/**
 * Parameters for estimating transaction fees.
 */
export interface EstimateTransactionFeeRequest {
    /** Base64-encoded transaction to estimate fees for */
    transaction: string;
    /** Mint address of the token to calculate fees in */
    fee_token?: string;
    /** Optional signer address for the transaction */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
}

/**
 * Parameters for estimating bundle fees.
 */
export interface EstimateBundleFeeRequest {
    /** Array of base64-encoded transactions to estimate fees for */
    transactions: string[];
    /** Mint address of the token to calculate fees in */
    fee_token?: string;
    /** Optional signer address for the transactions */
    signer_key?: string;
    /** Optional signer verification during transaction simulation (defaults to false) */
    sig_verify?: boolean;
    /** Optional indices of transactions to estimate fees for (defaults to all if not specified) */
    sign_only_indices?: number[];
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
 * The transaction is unsigned.
 * @deprecated Use `getPaymentInstruction` instead for fee payment flows.
 */
export interface TransferTransactionResponse {
    /** Base64-encoded unsigned transaction */
    transaction: string;
    /** Base64-encoded unsigned message */
    message: string;
    /** Recent blockhash used in the transaction */
    blockhash: string;
    /** Public key of the Kora signer (fee payer) */
    signer_pubkey: string;
    /** Parsed instructions from the transaction message */
    instructions: Instruction[];
}

/**
 * Response from signing a transaction.
 */
export interface SignTransactionResponse {
    /** Base64-encoded signed transaction */
    signed_transaction: string;
    /** Public key of the signer used to sign the transaction */
    signer_pubkey: string;
}

/**
 * Response from signing and sending a transaction.
 */
export interface SignAndSendTransactionResponse {
    /** Transaction signature */
    signature: string;
    /** Base64-encoded signed transaction */
    signed_transaction: string;
    /** Public key of the signer used to send the transaction */
    signer_pubkey: string;
}

/**
 * Response from signing a bundle of transactions.
 */
export interface SignBundleResponse {
    /** Array of base64-encoded signed transactions */
    signed_transactions: string[];
    /** Public key of the signer used to sign the transactions */
    signer_pubkey: string;
}

/**
 * Response from signing and sending a bundle of transactions via Jito.
 */
export interface SignAndSendBundleResponse {
    /** Array of base64-encoded signed transactions */
    signed_transactions: string[];
    /** Public key of the signer used to sign the transactions */
    signer_pubkey: string;
    /** UUID of the submitted Jito bundle */
    bundle_uuid: string;
}

/**
 * Response containing the latest blockhash.
 */
export interface GetBlockhashResponse {
    /** Base58-encoded blockhash */
    blockhash: string;
}

/**
 * Response containing the server version.
 */
export interface GetVersionResponse {
    /** Server version string */
    version: string;
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
    fee_in_token?: number;
    /** Public key of the signer used to estimate the fee */
    signer_pubkey: string;
    /** Public key of the payment destination */
    payment_address: string;
}

/**
 * Response containing estimated bundle fees.
 */
export interface EstimateBundleFeeResponse {
    /** Total bundle fee in lamports across all transactions */
    fee_in_lamports: number;
    /**
     * Total bundle fee in the requested token (in decimals value of the token, e.g. 10^6 for USDC)
     */
    fee_in_token?: number;
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
    /** Whether the estimate_bundle_fee method is enabled (requires bundle.enabled = true) */
    estimate_bundle_fee: boolean;
    /** Whether the get_supported_tokens method is enabled */
    get_supported_tokens: boolean;
    /** Whether the get_payer_signer method is enabled */
    get_payer_signer: boolean;
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
    /** Whether the get_version method is enabled */
    get_version: boolean;
    /** Whether the sign_and_send_bundle method is enabled (requires bundle.enabled = true) */
    sign_and_send_bundle: boolean;
    /** Whether the sign_bundle method is enabled (requires bundle.enabled = true) */
    sign_bundle: boolean;
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
 * Nonce instruction policy
 */
export interface NonceInstructionPolicy {
    /** Allow fee payer to initialize nonce accounts */
    allow_initialize: boolean;
    /** Allow fee payer to advance nonce accounts */
    allow_advance: boolean;
    /** Allow fee payer to authorize nonce accounts */
    allow_authorize: boolean;
    /** Allow fee payer to withdraw from nonce accounts */
    allow_withdraw: boolean;
}

/**
 * System instruction policy
 */
export interface SystemInstructionPolicy {
    /** Allow fee payer to be the sender in System Transfer/TransferWithSeed */
    allow_transfer: boolean;
    /** Allow fee payer to be the authority in System Assign/AssignWithSeed */
    allow_assign: boolean;
    /** Allow fee payer to be the payer in System CreateAccount/CreateAccountWithSeed */
    allow_create_account: boolean;
    /** Allow fee payer to be the account in System Allocate/AllocateWithSeed */
    allow_allocate: boolean;
    /** Nested policy for nonce account operations */
    nonce: NonceInstructionPolicy;
}

/**
 * SPL Token instruction policy
 */
export interface SplTokenInstructionPolicy {
    /** Allow fee payer to be source in SPL token transfers */
    allow_transfer: boolean;
    /** Allow fee payer to burn SPL tokens */
    allow_burn: boolean;
    /** Allow fee payer to close SPL token accounts */
    allow_close_account: boolean;
    /** Allow fee payer to approve SPL token delegates */
    allow_approve: boolean;
    /** Allow fee payer to revoke SPL token delegates */
    allow_revoke: boolean;
    /** Allow fee payer to set authority on SPL token accounts */
    allow_set_authority: boolean;
    /** Allow fee payer to mint SPL tokens */
    allow_mint_to: boolean;
    /** Allow fee payer to initialize SPL token mints */
    allow_initialize_mint: boolean;
    /** Allow fee payer to initialize SPL token accounts */
    allow_initialize_account: boolean;
    /** Allow fee payer to initialize SPL multisig accounts */
    allow_initialize_multisig: boolean;
    /** Allow fee payer to freeze SPL token accounts */
    allow_freeze_account: boolean;
    /** Allow fee payer to thaw SPL token accounts */
    allow_thaw_account: boolean;
}

/**
 * Token2022 instruction policy
 */
export interface Token2022InstructionPolicy {
    /** Allow fee payer to be source in Token2022 transfers */
    allow_transfer: boolean;
    /** Allow fee payer to burn Token2022 tokens */
    allow_burn: boolean;
    /** Allow fee payer to close Token2022 accounts */
    allow_close_account: boolean;
    /** Allow fee payer to approve Token2022 delegates */
    allow_approve: boolean;
    /** Allow fee payer to revoke Token2022 delegates */
    allow_revoke: boolean;
    /** Allow fee payer to set authority on Token2022 accounts */
    allow_set_authority: boolean;
    /** Allow fee payer to mint Token2022 tokens */
    allow_mint_to: boolean;
    /** Allow fee payer to initialize Token2022 mints */
    allow_initialize_mint: boolean;
    /** Allow fee payer to initialize Token2022 accounts */
    allow_initialize_account: boolean;
    /** Allow fee payer to initialize Token2022 multisig accounts */
    allow_initialize_multisig: boolean;
    /** Allow fee payer to freeze Token2022 accounts */
    allow_freeze_account: boolean;
    /** Allow fee payer to thaw Token2022 accounts */
    allow_thaw_account: boolean;
}

/**
 * Policy controlling what actions the fee payer can perform.
 */
export interface FeePayerPolicy {
    /** System program instruction policies */
    system: SystemInstructionPolicy;
    /** SPL Token program instruction policies */
    spl_token: SplTokenInstructionPolicy;
    /** Token2022 program instruction policies */
    token_2022: Token2022InstructionPolicy;
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

/**
 * Plugin Types - Kit-typed responses for the Kora plugin
 */

import type {
    Address,
    Blockhash,
    Signature,
    Instruction as KitInstruction,
    Base64EncodedWireTransaction,
} from '@solana/kit';

/** Configuration options for the Kora Kit plugin */
export interface KoraPluginConfig {
    /** Kora RPC endpoint URL */
    endpoint: string;
    /** Optional API key for authentication */
    apiKey?: string;
    /** Optional HMAC secret for signature-based authentication */
    hmacSecret?: string;
}

/** Plugin response for getPayerSigner with Kit Address types */
export interface KitPayerSignerResponse {
    /** Public key of the payer signer */
    signer_address: Address;
    /** Public key of the payment destination */
    payment_address: Address;
}

/** Plugin response for getBlockhash with Kit Blockhash type */
export interface KitBlockhashResponse {
    /** Base58-encoded blockhash */
    blockhash: Blockhash;
}

/** Plugin response for getSupportedTokens with Kit Address types */
export interface KitSupportedTokensResponse {
    /** Array of supported token mint addresses */
    tokens: Address[];
}

/** Plugin response for estimateTransactionFee with Kit Address types */
export interface KitEstimateFeeResponse {
    /** Transaction fee in lamports */
    fee_in_lamports: number;
    /** Transaction fee in the requested token */
    fee_in_token?: number;
    /** Public key of the signer used to estimate the fee */
    signer_pubkey: Address;
    /** Public key of the payment destination */
    payment_address: Address;
}

/** Plugin response for signTransaction with Kit types */
export interface KitSignTransactionResponse {
    /** Base64-encoded signed transaction */
    signed_transaction: Base64EncodedWireTransaction;
    /** Public key of the signer used to sign the transaction */
    signer_pubkey: Address;
}

/** Plugin response for signAndSendTransaction with Kit types */
export interface KitSignAndSendTransactionResponse {
    /** Transaction signature */
    signature: Signature;
    /** Base64-encoded signed transaction */
    signed_transaction: Base64EncodedWireTransaction;
    /** Public key of the signer used to send the transaction */
    signer_pubkey: Address;
}

/** Plugin response for transferTransaction with Kit types */
export interface KitTransferTransactionResponse {
    /** Base64-encoded signed transaction */
    transaction: Base64EncodedWireTransaction;
    /** Base64-encoded message */
    message: string;
    /** Recent blockhash used in the transaction */
    blockhash: Blockhash;
    /** Public key of the signer used to send the transaction */
    signer_pubkey: Address;
    /** Parsed instructions from the transaction message */
    instructions: KitInstruction[];
}

/** Plugin response for getPaymentInstruction with Kit types */
export interface KitPaymentInstructionResponse {
    /** Base64-encoded original transaction */
    original_transaction: Base64EncodedWireTransaction;
    /** Payment instruction */
    payment_instruction: KitInstruction;
    /** Payment amount in the requested token */
    payment_amount: number;
    /** Mint address of the token used for payment */
    payment_token: Address;
    /** Public key of the payment destination */
    payment_address: Address;
    /** Public key of the payer signer */
    signer_address: Address;
}

/** Plugin response for getConfig with Kit Address types */
export interface KitConfigResponse {
    /** Array of public keys of the fee payer accounts (signer pool) */
    fee_payers: Address[];
    /** Validation rules and constraints */
    validation_config: KitValidationConfig;
    /** Enabled methods */
    enabled_methods: EnabledMethods;
}

/** Plugin response for estimateBundleFee with Kit types */
export interface KitEstimateBundleFeeResponse {
    /** Total bundle fee in lamports across all transactions */
    fee_in_lamports: number;
    /** Total bundle fee in the requested token */
    fee_in_token?: number;
    /** Public key of the signer used to estimate the fee */
    signer_pubkey: Address;
    /** Public key of the payment destination */
    payment_address: Address;
}

/** Plugin response for signBundle with Kit types */
export interface KitSignBundleResponse {
    /** Array of base64-encoded signed transactions */
    signed_transactions: Base64EncodedWireTransaction[];
    /** Public key of the signer used to sign the transactions */
    signer_pubkey: Address;
}

/** Plugin response for signAndSendBundle with Kit types */
export interface KitSignAndSendBundleResponse {
    /** Array of base64-encoded signed transactions */
    signed_transactions: Base64EncodedWireTransaction[];
    /** Public key of the signer used to sign the transactions */
    signer_pubkey: Address;
    /** UUID of the submitted Jito bundle */
    bundle_uuid: string;
}

/** Plugin validation config with Kit Address types */
export interface KitValidationConfig {
    /** Maximum allowed transaction value in lamports */
    max_allowed_lamports: number;
    /** Maximum number of signatures allowed per transaction */
    max_signatures: number;
    /** Price oracle source for token conversions */
    price_source: PriceSource;
    /** List of allowed Solana program IDs */
    allowed_programs: Address[];
    /** List of allowed token mint addresses for fee payment */
    allowed_tokens: Address[];
    /** List of SPL tokens accepted for paid transactions */
    allowed_spl_paid_tokens: Address[];
    /** List of blocked account addresses */
    disallowed_accounts: Address[];
    /** Policy controlling fee payer permissions */
    fee_payer_policy: FeePayerPolicy;
    /** Pricing model configuration */
    price: PriceConfig;
    /** Token2022 configuration */
    token2022: Token2022Config;
}
