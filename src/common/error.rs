use jsonrpsee::{core::Error as RpcError, types::error::CallError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum KoraError {
    #[error("Account {0} does not exist")]
    AccountDoesNotExistError(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Signer error: {0}")]
    SignerError(String),
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl From<KoraError> for RpcError {
    fn from(err: KoraError) -> Self {
        match err {
            KoraError::AccountDoesNotExistError(msg) => {
                invalid_request(KoraError::AccountDoesNotExistError(msg))
            }
            KoraError::RpcError(msg) => invalid_request(KoraError::RpcError(msg)),
            KoraError::SignerError(msg) => invalid_request(KoraError::SignerError(msg)),
            KoraError::InternalServerError(msg) => {
                internal_server_error(KoraError::InternalServerError(msg))
            }
            KoraError::InvalidTransaction(msg) => {
                invalid_request(KoraError::InvalidTransaction(msg))
            }
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
