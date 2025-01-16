use jsonrpsee::{core::Error as RpcError, types::error::CallError};
use serde::{Deserialize, Serialize};
use solana_client::client_error::ClientError;
use solana_sdk::signature::SignerError;
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

impl From<Box<dyn std::error::Error>> for KoraError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        KoraError::InternalServerError(e.to_string())
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