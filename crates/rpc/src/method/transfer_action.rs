use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use kora_lib::config::ValidationConfig;

use super::transfer_transaction::{TransferTransactionRequest, TransferTransactionResponse};

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionGetResponse {
    #[serde(rename = "type")]
    pub action_type: String,
    pub icon: String,
    pub title: String,
    pub description: String,
    pub label: String,
    pub links: Option<ActionLinks>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionLinks {
    pub actions: Vec<LinkedAction>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkedAction {
    pub label: String,
    pub href: String,
    pub parameters: Vec<ActionParameter>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionParameter {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ActionPostRequest {
    pub account: String,
}

pub async fn get_transfer_action() -> ActionGetResponse {
    ActionGetResponse {
        action_type: "action".to_string(),
        icon: "https://solana.com/favicon.ico".to_string(), // Replace with actual icon
        title: "Kora Transfer".to_string(),
        description: "Transfer tokens between accounts".to_string(),
        label: "Transfer".to_string(),
        links: Some(ActionLinks {
            actions: vec![LinkedAction {
                label: "Transfer Tokens".to_string(),
                href: "/api/actions/transfer".to_string(),
                parameters: vec![
                    ActionParameter {
                        name: "amount".to_string(),
                        label: "Amount".to_string(),
                        param_type: "number".to_string(),
                        required: true,
                    },
                    ActionParameter {
                        name: "token".to_string(),
                        label: "Token Address".to_string(),
                        param_type: "text".to_string(),
                        required: true,
                    },
                    ActionParameter {
                        name: "destination".to_string(),
                        label: "Destination Address".to_string(),
                        param_type: "text".to_string(),
                        required: true,
                    },
                ],
            }],
        }),
    }
}

pub async fn post_transfer_action(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: ActionPostRequest,
    amount: u64,
    token: String,
    destination: String,
) -> Result<TransferTransactionResponse, kora_lib::KoraError> {
    let transfer_request = TransferTransactionRequest {
        amount,
        token,
        source: request.account,
        destination,
    };

    super::transfer_transaction::transfer_transaction(rpc_client, validation, transfer_request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use std::sync::Arc;
    use kora_lib::config::ValidationConfig;

    #[tokio::test]
    async fn test_get_transfer_action() {
        let response = get_transfer_action().await;
        assert_eq!(response.action_type, "action");
        assert_eq!(response.title, "Kora Transfer");
        assert_eq!(response.label, "Transfer");
        
        let links = response.links.unwrap();
        assert_eq!(links.actions.len(), 1);
        
        let action = &links.actions[0];
        assert_eq!(action.label, "Transfer Tokens");
        assert_eq!(action.parameters.len(), 3);
        
        let params: Vec<&str> = action.parameters.iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(params.contains(&"amount"));
        assert!(params.contains(&"token"));
        assert!(params.contains(&"destination"));
    }

    #[tokio::test]
    async fn test_post_transfer_action() {
        let rpc_client = Arc::new(RpcClient::new("http://localhost:8899".to_string()));
        let validation = ValidationConfig {
            allowed_tokens: vec![],
            disallowed_accounts: vec![],
            max_allowed_lamports: 0,
            max_signatures: 0,
            allowed_programs: vec![],
            allowed_spl_paid_tokens: vec![],
        };
        
        let request = ActionPostRequest {
            account: "11111111111111111111111111111111".to_string(),
        };
        
        let result = post_transfer_action(
            &rpc_client,
            &validation,
            request,
            1000000,
            "So11111111111111111111111111111111111111112".to_string(),
            "11111111111111111111111111111112".to_string(),
        ).await;
        
        // This will fail in tests since we're using a dummy RPC client
        assert!(result.is_err());
    }
} 