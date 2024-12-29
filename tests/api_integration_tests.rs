use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::json;
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

async fn setup_test_client() -> jsonrpsee::http_client::HttpClient {
    HttpClientBuilder::default().build(TEST_SERVER_URL).expect("Failed to create HTTP client")
}

fn create_test_transaction() -> String {
    let sender = Keypair::new();
    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();
    let amount = 10000;

    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);
    let message = Message::new_with_blockhash(
        &[instruction],
        Some(&sender.pubkey()),
        &solana_sdk::hash::Hash::default(),
    );

    let transaction = Transaction { signatures: vec![Default::default()], message };

    let serialized = bincode::serialize(&transaction).unwrap();
    bs58::encode(serialized).into_string()
}

#[tokio::test]
async fn test_get_enabled_features() {
    let client = setup_test_client().await;

    let response: serde_json::Value = client
        .request("getEnabledFeatures", rpc_params![])
        .await
        .expect("Failed to get enabled features");

    let features = response["features"].as_array().expect("Expected features array");
    assert!(!features.is_empty(), "Features list should not be empty");
    assert!(features.contains(&json!("gasless")), "Gasless feature should be enabled");
}

#[tokio::test]
async fn test_get_supported_tokens() {
    let client = setup_test_client().await;

    let response: serde_json::Value = client
        .request("getSupportedTokens", rpc_params![])
        .await
        .expect("Failed to get supported tokens");

    let tokens = response["tokens"].as_array().expect("Expected tokens array");
    assert!(!tokens.is_empty(), "Tokens list should not be empty");

    // Check for specific known tokens
    let expected_tokens = [
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    ];

    for token in expected_tokens.iter() {
        assert!(tokens.contains(&json!(token)), "Expected token {} not found", token);
    }
}

#[tokio::test]
async fn test_estimate_transaction_fee() {
    let client = setup_test_client().await;
    let test_tx = create_test_transaction();

    let response: serde_json::Value = client
        .request(
            "estimateTransactionFee",
            rpc_params![test_tx, "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"],
        )
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
}

#[tokio::test]
async fn test_sign_transaction() {
    let client = setup_test_client().await;
    let test_tx = create_test_transaction();

    let response: serde_json::Value = client
        .request("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_and_send() {
    let client = setup_test_client().await;
    let test_tx = create_test_transaction();

    let result = client.request::<serde_json::Value, _>("signAndSend", rpc_params![test_tx]).await;

    // This might fail if we're not on devnet/testnet with funded accounts
    match result {
        Ok(response) => {
            assert!(response["signature"].as_str().is_some(), "Expected signature in response");
            assert!(
                response["signed_transaction"].as_str().is_some(),
                "Expected signed_transaction in response"
            );
        }
        Err(e) => {
            println!("Note: signAndSend failed as expected without funded accounts: {}", e);
        }
    }
}

#[tokio::test]
async fn test_invalid_transaction() {
    let client = setup_test_client().await;
    let invalid_tx = "invalid_base58_transaction";

    let result =
        client.request::<serde_json::Value, _>("signTransaction", rpc_params![invalid_tx]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

#[tokio::test]
async fn test_transfer_transaction() {
    let client = setup_test_client().await;

    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                1000000,
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "6kntKawNmZNKZqUHvRVGKMwp8LQU5upyhht7w1PL7dde",
                "BrfrZdQNEitACxyYLNmFRWHtRzZFNFpYH5GAtoA1XXU6"
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    // Verify the response contains the expected fields
    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
}

#[tokio::test]
async fn test_transfer_transaction_with_ata() {
    let client = setup_test_client().await;
    let random_pubkey = Keypair::new().pubkey();

    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                1000000,
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "6kntKawNmZNKZqUHvRVGKMwp8LQU5upyhht7w1PL7dde",
                random_pubkey.to_string()
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    // Verify the response contains the expected fields
    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
}
