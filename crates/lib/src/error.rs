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

    #[error("Thread safety error: {0}")]
    ThreadSafetyError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl From<ClientError> for KoraError {
    fn from(e: ClientError) -> Self {
        KoraError::RpcError(e.to_string())
    }
}

impl From<SignerError> for KoraError {
    fn from(e: SignerError) -> Self {
        KoraError::SigningError(e.to_string())
    }
}

impl From<bincode::Error> for KoraError {
    fn from(e: bincode::Error) -> Self {
        KoraError::SerializationError(e.to_string())
    }
}

impl From<bs58::decode::Error> for KoraError {
    fn from(e: bs58::decode::Error) -> Self {
        KoraError::SerializationError(e.to_string())
    }
}

impl From<bs58::encode::Error> for KoraError {
    fn from(e: bs58::encode::Error) -> Self {
        KoraError::SerializationError(e.to_string())
    }
}

impl From<std::io::Error> for KoraError {
    fn from(e: std::io::Error) -> Self {
        KoraError::InternalServerError(e.to_string())
    }
}

impl From<Box<dyn StdError>> for KoraError {
    fn from(e: Box<dyn StdError>) -> Self {
        KoraError::InternalServerError(e.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync>> for KoraError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        KoraError::InternalServerError(e.to_string())
    }
}

impl From<ProgramError> for KoraError {
    fn from(err: ProgramError) -> Self {
        KoraError::InvalidTransaction(err.to_string())
    }
}

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

impl From<anyhow::Error> for KoraError {
    fn from(err: anyhow::Error) -> Self {
        KoraError::SigningError(err.to_string())
    }
}

impl From<kora_privy::PrivyError> for KoraError {
    fn from(err: kora_privy::PrivyError) -> Self {
        KoraError::SigningError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_error_conversions() {
        let client_error =
            ClientError::from(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let kora_error: KoraError = client_error.into();
        assert!(matches!(kora_error, KoraError::RpcError(_)));

        let signer_error = SignerError::Custom("test".to_string());
        let kora_error: KoraError = signer_error.into();
        assert!(matches!(kora_error, KoraError::SigningError(_)));

        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let kora_error: KoraError = io_error.into();
        assert!(matches!(kora_error, KoraError::InternalServerError(_)));
    }
}
