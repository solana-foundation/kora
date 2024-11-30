use jsonrpsee::core::Error as RpcError;
use jsonrpsee::types::error::CallError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum KoraError {
    #[error("Account {0} does not exist")]
    AccountDoesNotExist(String),
    #[error("Not enough funds in account")]
    NotEnoughFunds,
    #[error("RPC error: {0}")]
    RpcError(String),
}

impl From<KoraError> for RpcError {
    fn from(err: KoraError) -> Self {
        match err {
            KoraError::AccountDoesNotExist(account) => {
                invalid_request(KoraError::AccountDoesNotExist(account))
            }
            KoraError::NotEnoughFunds => invalid_server_error(),
            KoraError::RpcError(e) => invalid_request(KoraError::RpcError(e)),
        }
    }
}

pub(crate) fn invalid_request(e: KoraError) -> RpcError {
    RpcError::Call(CallError::from_std_error(e))
}

pub(crate) fn invalid_server_error() -> RpcError {
    RpcError::Call(CallError::Failed(anyhow::anyhow!("Server error: insufficient funds")))
}
