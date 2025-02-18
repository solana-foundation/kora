use super::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::str::FromStr;

#[tokio::test]
async fn test_get_transfer_metadata() {
    let metadata = get_transfer_metadata().await;
    assert_eq!(metadata.name, "Transfer");
    assert!(!metadata.disabled);
    assert!(metadata.error_message.is_none());
}

#[tokio::test]
async fn test_handle_transfer_action() {
    let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
    let validation = ValidationConfig::default();
    
    let source = Keypair::new();
    let destination = Pubkey::new_unique();
    
    let request = TransferActionRequest {
        amount: 1000000,
        token: "So11111111111111111111111111111111111111112".to_string(), // SOL
        source: source.pubkey().to_string(),
        destination: destination.to_string(),
    };

    let result = handle_transfer_action(&rpc_client, &validation, request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    assert!(!response.transaction.is_empty());
    assert!(!response.message.is_empty());
    assert!(!response.blockhash.is_empty());
} 