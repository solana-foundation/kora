use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use kora_lib::{
    config::ValidationConfig,
    error::KoraError,
};

use crate::method::transfer_transaction::{
    TransferTransactionRequest, TransferTransactionResponse, transfer_transaction,
};

#[derive(Debug, Deserialize)]
pub struct TransferActionRequest {
    pub amount: u64,
    pub token: String,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Serialize)]
pub struct TransferActionMetadata {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub disabled: bool,
    pub error_message: Option<String>,
}

pub async fn get_transfer_metadata() -> TransferActionMetadata {
    TransferActionMetadata {
        name: "Transfer".to_string(),
        description: "Transfer tokens between accounts".to_string(),
        icon: "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".to_string(),
        disabled: false,
        error_message: None,
    }
}

pub async fn handle_transfer_action(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: TransferActionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let transfer_request = TransferTransactionRequest {
        amount: request.amount,
        token: request.token,
        source: request.source,
        destination: request.destination,
    };

    transfer_transaction(rpc_client, validation, transfer_request).await
} 