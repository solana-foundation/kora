
use kora_lib::config::ValidationConfig;
use kora_rpc::method::transfer_action::{
    get_transfer_action, post_transfer_action, TransferActionRequest,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

#[tokio::test]
async fn test_get_transfer_action() {
    let response = get_transfer_action().await.unwrap();
    
    assert_eq!(response.title, "Transfer Tokens");
    assert_eq!(response.action_type, "action");
    assert_eq!(response.links.actions.len(), 1);
    
    let action = &response.links.actions[0];
    assert_eq!(action.label, "Transfer");
    assert_eq!(action.href, "/actions/transfer");
    assert_eq!(action.parameters.len(), 3);
    
    // Check parameters
    let amount_param = action.parameters.iter().find(|p| p.name == "amount").unwrap();
    assert_eq!(amount_param.param_type, "text");
    assert!(amount_param.required);
    
    let token_param = action.parameters.iter().find(|p| p.name == "token").unwrap();
    assert_eq!(token_param.param_type, "text");
    assert!(token_param.required);
    
    let destination_param = action.parameters.iter().find(|p| p.name == "destination").unwrap();
    assert_eq!(destination_param.param_type, "text");
    assert!(destination_param.required);
}

#[tokio::test]
async fn test_post_transfer_action_validation() {
    let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
    
    let validation_config = ValidationConfig {
        max_allowed_lamports: 1000000000,
        max_signatures: 10,
        allowed_programs: vec![
            "11111111111111111111111111111112".to_string(), // System program
        ],
        allowed_tokens: vec![
            "So11111111111111111111111111111111111111112".to_string(), // Native SOL
        ],
        allowed_spl_paid_tokens: vec![],
        disallowed_accounts: vec![],
        price_source: kora_lib::oracle::PriceSource::Jupiter,
    };
    
    // Test with invalid amount format
    let invalid_request = TransferActionRequest {
        account: "5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht".to_string(),
        amount: "invalid_amount".to_string(),
        token: "So11111111111111111111111111111111111111112".to_string(),
        destination: "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM".to_string(),
    };
    
    let result = post_transfer_action(&rpc_client, &validation_config, invalid_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_post_transfer_action_valid_format() {
    let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
    
    let validation_config = ValidationConfig {
        max_allowed_lamports: 1000000000,
        max_signatures: 10,
        allowed_programs: vec![
            "11111111111111111111111111111112".to_string(), // System program
        ],
        allowed_tokens: vec![
            "So11111111111111111111111111111111111111112".to_string(), // Native SOL
        ],
        allowed_spl_paid_tokens: vec![],
        disallowed_accounts: vec![],
        price_source: kora_lib::oracle::PriceSource::Jupiter,
    };
    
    let valid_request = TransferActionRequest {
        account: "5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht".to_string(),
        amount: "1.5".to_string(),
        token: "So11111111111111111111111111111111111111112".to_string(),
        destination: "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM".to_string(),
    };
    
    // This might fail due to network issues or validation, but the format should be correct
    let result = post_transfer_action(&rpc_client, &validation_config, valid_request).await;
    
    // We're mainly testing that the parsing works correctly
    // The actual transaction creation might fail due to network or validation issues
    match result {
        Ok(response) => {
            assert!(!response.transaction.is_empty());
            assert!(response.message.is_some());
        }
        Err(e) => {
            // Error is expected in test environment, but should not be a parsing error
            assert!(!e.to_string().contains("Invalid amount format"));
        }
    }
}
