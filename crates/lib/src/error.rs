use crate::{bundle::BundleError, sanitize::sanitize_message};
use jsonrpsee::{core::Error as RpcError, types::error::CallError};
use serde::{Deserialize, Serialize};
use solana_client::client_error::ClientError;
use solana_program::program_error::ProgramError;
use solana_sdk::signature::SignerError;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum KoraError {
    #[error("Account {0} not found")]
    AccountNotFound(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Transaction execution failed: {0}")]
    TransactionExecutionFailed(String),

    #[error("Fee estimation failed: {0}")]
    FeeEstimationFailed(String),

    #[error("Token {0} is not supported for fee payment")]
    UnsupportedFeeToken(String),

    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Internal error: {0}")]
    InternalServerError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Swap error: {0}")]
    SwapError(String),

    #[error("Token operation failed: {0}")]
    TokenOperationError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Usage limit exceeded: {0}")]
    UsageLimitExceeded(String),

    #[error("Invalid configuration for Kora")]
    ConfigError,

    #[error("Jito error: {0}")]
    JitoError(String),

    #[error("reCAPTCHA error: {0}")]
    RecaptchaError(String),
}

impl From<ClientError> for KoraError {
    fn from(e: ClientError) -> Self {
        let error_string = e.to_string();
        let sanitized_error_string = sanitize_message(&error_string);
        if error_string.contains("AccountNotFound")
            || error_string.contains("could not find account")
        {
            #[cfg(feature = "unsafe-debug")]
            {
                KoraError::AccountNotFound(error_string)
            }
            #[cfg(not(feature = "unsafe-debug"))]
            {
                KoraError::AccountNotFound(sanitized_error_string)
            }
        } else {
            #[cfg(feature = "unsafe-debug")]
            {
                KoraError::RpcError(error_string)
            }
            #[cfg(not(feature = "unsafe-debug"))]
            {
                KoraError::RpcError(sanitized_error_string)
            }
        }
    }
}

macro_rules! impl_kora_error_from {
    ($source:ty => $variant:ident) => {
        impl From<$source> for KoraError {
            fn from(e: $source) -> Self {
                #[cfg(feature = "unsafe-debug")]
                {
                    KoraError::$variant(e.to_string())
                }
                #[cfg(not(feature = "unsafe-debug"))]
                {
                    KoraError::$variant(sanitize_message(&e.to_string()))
                }
            }
        }
    };
}

impl_kora_error_from!(SignerError => SigningError);
impl_kora_error_from!(bincode::Error => SerializationError);
impl_kora_error_from!(bs58::decode::Error => SerializationError);
impl_kora_error_from!(bs58::encode::Error => SerializationError);
impl_kora_error_from!(std::io::Error => InternalServerError);
impl_kora_error_from!(Box<dyn StdError> => InternalServerError);
impl_kora_error_from!(Box<dyn StdError + Send + Sync> => InternalServerError);
impl_kora_error_from!(ProgramError => InvalidTransaction);

impl From<KoraError> for RpcError {
    fn from(err: KoraError) -> Self {
        match err {
            KoraError::AccountNotFound(_)
            | KoraError::InvalidTransaction(_)
            | KoraError::ValidationError(_)
            | KoraError::UnsupportedFeeToken(_)
            | KoraError::InsufficientFunds(_) => invalid_request(err),

            KoraError::InternalServerError(_) | KoraError::SerializationError(_) => {
                internal_server_error(err)
            }

            _ => invalid_request(err),
        }
    }
}

pub fn invalid_request(e: KoraError) -> RpcError {
    RpcError::Call(CallError::from_std_error(e))
}

pub fn internal_server_error(e: KoraError) -> RpcError {
    RpcError::Call(CallError::from_std_error(e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoraResponse<T> {
    pub data: Option<T>,
    pub error: Option<KoraError>,
}

impl<T> KoraResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { data: Some(data), error: None }
    }

    pub fn err(error: KoraError) -> Self {
        Self { data: None, error: Some(error) }
    }

    pub fn from_result(result: Result<T, KoraError>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(error) => Self::err(error),
        }
    }
}

// Extension trait for Result<T, E> to convert to KoraResponse
pub trait IntoKoraResponse<T> {
    fn into_response(self) -> KoraResponse<T>;
}

impl<T, E: Into<KoraError>> IntoKoraResponse<T> for Result<T, E> {
    fn into_response(self) -> KoraResponse<T> {
        match self {
            Ok(data) => KoraResponse::ok(data),
            Err(e) => KoraResponse::err(e.into()),
        }
    }
}

impl_kora_error_from!(anyhow::Error => SigningError);
impl_kora_error_from!(solana_keychain::SignerError => SigningError);

impl From<BundleError> for KoraError {
    fn from(err: BundleError) -> Self {
        match err {
            BundleError::Jito(_) => KoraError::JitoError(err.to_string()),
            _ => KoraError::InvalidTransaction(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_error::ProgramError;
    use std::error::Error as StdError;

    #[test]
    fn test_kora_response_ok() {
        let response = KoraResponse::ok(42);
        assert_eq!(response.data, Some(42));
        assert_eq!(response.error, None);
    }

    #[test]
    fn test_kora_response_err() {
        let error = KoraError::AccountNotFound("test_account".to_string());
        let response: KoraResponse<()> = KoraResponse::err(error.clone());
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some(error));
    }

    #[test]
    fn test_kora_response_from_result() {
        let ok_response = KoraResponse::from_result(Ok(42));
        assert_eq!(ok_response.data, Some(42));
        assert_eq!(ok_response.error, None);

        let error = KoraError::ValidationError("test error".to_string());
        let err_response: KoraResponse<i32> = KoraResponse::from_result(Err(error.clone()));
        assert_eq!(err_response.data, None);
        assert_eq!(err_response.error, Some(error));
    }

    #[test]
    fn test_into_kora_response() {
        let result: Result<i32, KoraError> = Ok(42);
        let response = result.into_response();
        assert_eq!(response.data, Some(42));
        assert_eq!(response.error, None);

        let error = KoraError::SwapError("swap failed".to_string());
        let result: Result<i32, KoraError> = Err(error.clone());
        let response = result.into_response();
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some(error));
    }

    #[test]
    fn test_client_error_conversion() {
        let client_error = ClientError::from(std::io::Error::other("test"));
        let kora_error: KoraError = client_error.into();
        assert!(matches!(kora_error, KoraError::RpcError(_)));
        // With sanitization, error message context is preserved unless it contains sensitive data
        if let KoraError::RpcError(msg) = kora_error {
            assert!(msg.contains("test"));
        }
    }

    #[test]
    fn test_signer_error_conversion() {
        let signer_error = SignerError::Custom("signing failed".to_string());
        let kora_error: KoraError = signer_error.into();
        assert!(matches!(kora_error, KoraError::SigningError(_)));
        // With sanitization, error message context is preserved unless it contains sensitive data
        if let KoraError::SigningError(msg) = kora_error {
            assert!(msg.contains("signing failed"));
        }
    }

    #[test]
    fn test_bincode_error_conversion() {
        let bincode_error = bincode::Error::from(bincode::ErrorKind::SizeLimit);
        let kora_error: KoraError = bincode_error.into();
        assert!(matches!(kora_error, KoraError::SerializationError(_)));
    }

    #[test]
    fn test_bs58_decode_error_conversion() {
        let bs58_error = bs58::decode::Error::InvalidCharacter { character: 'x', index: 0 };
        let kora_error: KoraError = bs58_error.into();
        assert!(matches!(kora_error, KoraError::SerializationError(_)));
    }

    #[test]
    fn test_bs58_encode_error_conversion() {
        let buffer_too_small_error = bs58::encode::Error::BufferTooSmall;
        let kora_error: KoraError = buffer_too_small_error.into();
        assert!(matches!(kora_error, KoraError::SerializationError(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::other("file not found");
        let kora_error: KoraError = io_error.into();
        assert!(matches!(kora_error, KoraError::InternalServerError(_)));
        // With sanitization, error message context is preserved unless it contains sensitive data
        if let KoraError::InternalServerError(msg) = kora_error {
            assert!(msg.contains("file not found"));
        }
    }

    #[test]
    fn test_boxed_error_conversion() {
        let error: Box<dyn StdError> = Box::new(std::io::Error::other("boxed error"));
        let kora_error: KoraError = error.into();
        assert!(matches!(kora_error, KoraError::InternalServerError(_)));
    }

    #[test]
    fn test_boxed_error_send_sync_conversion() {
        let error: Box<dyn StdError + Send + Sync> =
            Box::new(std::io::Error::other("boxed send sync error"));
        let kora_error: KoraError = error.into();
        assert!(matches!(kora_error, KoraError::InternalServerError(_)));
    }

    #[test]
    fn test_program_error_conversion() {
        let program_error = ProgramError::InvalidAccountData;
        let kora_error: KoraError = program_error.into();
        assert!(matches!(kora_error, KoraError::InvalidTransaction(_)));
        if let KoraError::InvalidTransaction(msg) = kora_error {
            // Just check that the error is converted properly, don't rely on specific formatting
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_error = anyhow::anyhow!("something went wrong");
        let kora_error: KoraError = anyhow_error.into();
        assert!(matches!(kora_error, KoraError::SigningError(_)));
        // With sanitization, error message context is preserved unless it contains sensitive data
        if let KoraError::SigningError(msg) = kora_error {
            assert!(msg.contains("something went wrong"));
        }
    }

    #[test]
    fn test_kora_error_to_rpc_error_invalid_request() {
        let test_cases = vec![
            KoraError::AccountNotFound("test".to_string()),
            KoraError::InvalidTransaction("test".to_string()),
            KoraError::ValidationError("test".to_string()),
            KoraError::UnsupportedFeeToken("test".to_string()),
            KoraError::InsufficientFunds("test".to_string()),
        ];

        for kora_error in test_cases {
            let rpc_error: RpcError = kora_error.into();
            assert!(matches!(rpc_error, RpcError::Call(_)));
        }
    }

    #[test]
    fn test_kora_error_to_rpc_error_internal_server() {
        let test_cases = vec![
            KoraError::InternalServerError("test".to_string()),
            KoraError::SerializationError("test".to_string()),
        ];

        for kora_error in test_cases {
            let rpc_error: RpcError = kora_error.into();
            assert!(matches!(rpc_error, RpcError::Call(_)));
        }
    }

    #[test]
    fn test_kora_error_to_rpc_error_default_case() {
        let other_errors = vec![
            KoraError::RpcError("test".to_string()),
            KoraError::SigningError("test".to_string()),
            KoraError::TransactionExecutionFailed("test".to_string()),
            KoraError::FeeEstimationFailed("test".to_string()),
            KoraError::SwapError("test".to_string()),
            KoraError::TokenOperationError("test".to_string()),
            KoraError::InvalidRequest("test".to_string()),
            KoraError::Unauthorized("test".to_string()),
            KoraError::RateLimitExceeded,
        ];

        for kora_error in other_errors {
            let rpc_error: RpcError = kora_error.into();
            assert!(matches!(rpc_error, RpcError::Call(_)));
        }
    }

    #[test]
    fn test_invalid_request_function() {
        let error = KoraError::ValidationError("invalid input".to_string());
        let rpc_error = invalid_request(error);
        assert!(matches!(rpc_error, RpcError::Call(_)));
    }

    #[test]
    fn test_internal_server_error_function() {
        let error = KoraError::InternalServerError("server panic".to_string());
        let rpc_error = internal_server_error(error);
        assert!(matches!(rpc_error, RpcError::Call(_)));
    }

    #[test]
    fn test_into_kora_response_with_different_error_types() {
        let io_result: Result<String, std::io::Error> = Err(std::io::Error::other("test"));
        let response = io_result.into_response();
        assert_eq!(response.data, None);
        assert!(matches!(response.error, Some(KoraError::InternalServerError(_))));

        let signer_result: Result<String, SignerError> =
            Err(SignerError::Custom("test".to_string()));
        let response = signer_result.into_response();
        assert_eq!(response.data, None);
        assert!(matches!(response.error, Some(KoraError::SigningError(_))));
    }

    #[test]
    fn test_kora_error_display() {
        let error = KoraError::AccountNotFound("test_account".to_string());
        let display_string = format!("{error}");
        assert_eq!(display_string, "Account test_account not found");

        let error = KoraError::RateLimitExceeded;
        let display_string = format!("{error}");
        assert_eq!(display_string, "Rate limit exceeded");
    }

    #[test]
    fn test_kora_error_debug() {
        let error = KoraError::ValidationError("test".to_string());
        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("ValidationError"));
    }

    #[test]
    fn test_kora_error_clone() {
        let error = KoraError::SwapError("original".to_string());
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_kora_response_serialization() {
        let response = KoraResponse::ok("test_data".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_data"));

        let error_response: KoraResponse<String> =
            KoraResponse::err(KoraError::ValidationError("test".to_string()));
        let error_json = serde_json::to_string(&error_response).unwrap();
        assert!(error_json.contains("ValidationError"));
    }
}
