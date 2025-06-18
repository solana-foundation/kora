
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use kora_lib::{config::ValidationConfig, KoraError};

use super::transfer_transaction::{transfer_transaction, TransferTransactionRequest};

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionMetadata {
    pub icon: String,
    pub title: String,
    pub description: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionParameter {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub pattern: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionGetResponse {
    pub icon: String,
    pub title: String,
    pub description: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub links: ActionLinks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionLinks {
    pub actions: Vec<ActionLink>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionLink {
    pub label: String,
    pub href: String,
    pub parameters: Vec<ActionParameter>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferActionRequest {
    pub account: String,
    pub amount: String,
    pub token: String,
    pub destination: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionPostResponse {
    pub transaction: String,
    pub message: Option<String>,
}

pub async fn get_transfer_action() -> Result<ActionGetResponse, KoraError> {
    Ok(ActionGetResponse {
        icon: "https://via.placeholder.com/512x512.png".to_string(),
        title: "Transfer Tokens".to_string(),
        description: "Transfer tokens from your wallet to another address".to_string(),
        label: "Transfer".to_string(),
        action_type: "action".to_string(),
        links: ActionLinks {
            actions: vec![ActionLink {
                label: "Transfer".to_string(),
                href: "/actions/transfer".to_string(),
                parameters: vec![
                    ActionParameter {
                        name: "amount".to_string(),
                        label: "Amount".to_string(),
                        param_type: "text".to_string(),
                        required: true,
                        pattern: Some(r"^\d+(\.\d+)?$".to_string()),
                        placeholder: Some("Enter amount".to_string()),
                    },
                    ActionParameter {
                        name: "token".to_string(),
                        label: "Token Address".to_string(),
                        param_type: "text".to_string(),
                        required: true,
                        pattern: Some(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$".to_string()),
                        placeholder: Some("Token mint address".to_string()),
                    },
                    ActionParameter {
                        name: "destination".to_string(),
                        label: "Destination Address".to_string(),
                        param_type: "text".to_string(),
                        required: true,
                        pattern: Some(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$".to_string()),
                        placeholder: Some("Recipient wallet address".to_string()),
                    },
                ],
            }],
        },
    })
}

pub async fn post_transfer_action(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: TransferActionRequest,
) -> Result<ActionPostResponse, KoraError> {
    // Parse amount - assume it's in token units and convert to lamports/smallest unit
    let amount = request
        .amount
        .parse::<f64>()
        .map_err(|_| KoraError::ValidationError("Invalid amount format".to_string()))?;
    
    // For simplicity, assume 6 decimals for most tokens (adjust as needed)
    let amount_lamports = (amount * 1_000_000.0) as u64;

    let transfer_request = TransferTransactionRequest {
        amount: amount_lamports,
        token: request.token,
        source: request.account,
        destination: request.destination,
    };

    let response = transfer_transaction(rpc_client, validation, transfer_request).await?;

    Ok(ActionPostResponse {
        transaction: response.transaction,
        message: Some("Transfer transaction created successfully".to_string()),
    })
}
