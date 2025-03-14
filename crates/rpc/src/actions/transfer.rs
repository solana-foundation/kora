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

#[derive(Debug, Serialize)]
pub struct TransferActionResponse {
    pub transaction: String,  // base64 encoded transaction
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<ActionLinks>,
}

#[derive(Debug, Serialize)]
pub struct ActionLinks {
    pub next: Option<NextActionLink>,
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
) -> Result<TransferActionResponse, KoraError> {
    let transfer_request = TransferTransactionRequest {
        amount: request.amount,
        token: request.token,
        source: request.source,
        destination: request.destination,
    };

    let tx_response = transfer_transaction(rpc_client, validation, transfer_request).await?;
    
    Ok(TransferActionResponse {
        transaction: tx_response.transaction,
        message: format!("Transfer {} tokens to {}", request.amount, request.destination),
        links: None
    })
} 