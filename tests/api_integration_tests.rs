use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc};

const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

fn get_rpc_url() -> String {
    std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string())
}

async fn setup_test_client() -> jsonrpsee::http_client::HttpClient {
    HttpClientBuilder::default().build(TEST_SERVER_URL).expect("Failed to create HTTP client")
}

async fn setup_rpc_client() -> Arc<RpcClient> {
    Arc::new(RpcClient::new(get_rpc_url()))
}

fn create_test_transaction() -> String {
    let sender = Keypair::new();
    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();
    let amount = 10;

    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);
    let message =
        Message::new_with_blockhash(&[instruction], None, &solana_sdk::hash::Hash::default());

    let transaction = Transaction { signatures: vec![Default::default()], message };

    let serialized = bincode::serialize(&transaction).unwrap();
    bs58::encode(serialized).into_string()
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
    let rpc_client = setup_rpc_client().await;

    let response: serde_json::Value = client
        .request("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base58 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    // Deserialize the transaction
    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    // Optional: Add assertions to verify simulation results
    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
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
    let rpc_client = setup_rpc_client().await;

    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                1,
                "11111111111111111111111111111111",
                "6kntKawNmZNKZqUHvRVGKMwp8LQU5upyhht7w1PL7dde",
                "BrfrZdQNEitACxyYLNmFRWHtRzZFNFpYH5GAtoA1XXU6"
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    // Verify the response contains the expected fields
    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    // Decode the base58 transaction string
    let transaction_string = response["transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    // Deserialize the transaction
    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    // Optional: Add assertions to verify simulation results
    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_transfer_transaction_with_ata() {
    let client = setup_test_client().await;
    let rpc_client = setup_rpc_client().await;
    let random_keypair = Keypair::new();
    let random_pubkey = random_keypair.pubkey();

    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                10,
                "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
                "J1NiBQHq1Q98HwB4xZCpekg66oXniqzW9vXJorZNuF9R",
                random_pubkey.to_string()
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    // Verify the response contains the expected fields
    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    // Decode the base58 transaction string
    let transaction_string = response["transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    // Deserialize the transaction
    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    // Will fail unless you sign tx before seding
    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_get_blockhash() {
    let client = setup_test_client().await;

    let response: serde_json::Value =
        client.request("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");
}

#[tokio::test]
async fn test_get_config() {
    let client = setup_test_client().await;

    let response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");
    assert!(response["fee_payer"].as_str().is_some(), "Expected fee_payer in response");
    assert!(
        response["validation_config"].as_object().is_some(),
        "Expected validation_config in response"
    );
}

// #[tokio::test]
async fn test_swap_to_sol() {
    // This will fail unless you set JUPITER_API_URL to a custom devnet url

    let client = setup_test_client().await;
    let rpc_client = setup_rpc_client().await;

    let response: serde_json::Value = client
        .request(
            "swapToSol",
            rpc_params![
                "6kntKawNmZNKZqUHvRVGKMwp8LQU5upyhht7w1PL7dde",
                100,
                "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
            ],
        )
        .await
        .expect("Failed to create swap to sol tx");

    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(response["transaction"].as_str().is_some(), "Expected transaction in response");

    // Decode the base58 transaction string
    let transaction_string = response["transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    // Deserialize the transaction
    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}
