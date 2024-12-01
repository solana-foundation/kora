use jsonrpsee::{core::Error as RpcError, types::error::CallError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum KoraError {
    #[error("Account {0} does not exist")]
    AccountDoesNotExist(String),
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("Signer error: {0}")]
    Signer(String),
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    #[error("Failed to estimate fee")]
    FeeEstimation,
    #[error("Unsupported fee token")]
    UnsupportedFeeToken,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl From<KoraError> for RpcError {
    fn from(err: KoraError) -> Self {
        match err {
            KoraError::AccountDoesNotExist(msg) => {
                invalid_request(KoraError::AccountDoesNotExist(msg))
            }
            KoraError::Rpc(msg) => invalid_request(KoraError::Rpc(msg)),
            KoraError::Signer(msg) => invalid_request(KoraError::Signer(msg)),
            KoraError::InternalServerError(msg) => {
                internal_server_error(KoraError::InternalServerError(msg))
            }
            KoraError::InvalidTransaction(msg) => {
                invalid_request(KoraError::InvalidTransaction(msg))
            }
            KoraError::FeeEstimation => invalid_request(KoraError::FeeEstimation),
            KoraError::UnsupportedFeeToken => invalid_request(KoraError::UnsupportedFeeToken),
            KoraError::TransactionFailed(msg) => invalid_request(KoraError::TransactionFailed(msg)),
            KoraError::InsufficientFunds => invalid_request(KoraError::InsufficientFunds),
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
}
