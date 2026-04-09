/**
 * Stable numeric error codes for RPC responses following JSON-RPC 2.0 spec (-32000 to -32099 for server errors).
 */
export enum KoraErrorCode {
    // Validation errors (-32000 to -32019)
    InvalidTransaction = -32000,
    ValidationError = -32001,
    UnsupportedFeeToken = -32002,
    InsufficientFunds = -32003,
    InvalidRequest = -32004,
    FeeEstimationFailed = -32005,
    TransactionExecutionFailed = -32006,

    // Signing errors (-32020 to -32029)
    SigningError = -32020,

    // Auth / Rate limiting (-32030 to -32039)
    RateLimitExceeded = -32030,
    UsageLimitExceeded = -32031,
    Unauthorized = -32032,

    // Token / Swap (-32040 to -32049)
    SwapError = -32040,
    TokenOperationError = -32041,

    // Account errors (-32050 to -32059)
    AccountNotFound = -32050,

    // External services (-32060 to -32069)
    JitoError = -32060,
    RecaptchaError = -32061,

    // Internal errors (-32090 to -32099)
    InternalServerError = -32090,
    ConfigError = -32091,
    SerializationError = -32092,
    RpcError = -32093,
}

/**
 * Structured data returned in the JSON-RPC error object's `data` field.
 */
export interface KoraErrorData {
    error_type: string;
    message: string;
}

/**
 * Represents a structured error returned by the Kora paymaster service.
 */
export class KoraError extends Error {
    /** The stable numeric error code */
    public readonly code: number;
    /** Optional structured data about the error */
    public readonly data?: KoraErrorData;

    constructor(code?: number, message?: string, data?: KoraErrorData) {
        const finalCode = code ?? -32603;
        const finalMessage = message ?? 'Unknown error';
        super(`Kora Error ${finalCode}: ${finalMessage}`);
        this.name = 'KoraError';
        this.code = finalCode;
        this.data = data;

        // Ensure proper stack trace in environments that support Error.captureStackTrace
        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, KoraError);
        }
    }
}
