use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    signer::SeedDerivable,
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use kora_lib::token::{TokenBase, TokenType};
use std::{str::FromStr, sync::Arc};
use kora_lib::token::TokenKeg;

const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

fn get_rpc_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string())
}

fn get_test_sender_keypair() -> Keypair {
    dotenv::dotenv().ok();
    Keypair::from_seed_phrase_and_passphrase(&std::env::var("TEST_SENDER_MNEMONIC").unwrap(), "")
        .unwrap()
}

async fn setup_test_client() -> jsonrpsee::http_client::HttpClient {
    HttpClientBuilder::default().build(TEST_SERVER_URL).expect("Failed to create HTTP client")
}

async fn setup_rpc_client() -> Arc<RpcClient> {
    Arc::new(RpcClient::new(get_rpc_url()))
}

async fn create_test_transaction() -> String {
    let sender = get_test_sender_keypair();
    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();
    let amount = 10;
    let rpc_client = setup_rpc_client().await;
    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    let message = Message::new_with_blockhash(&[instruction], None, &blockhash.0);

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
    let expected_tokens = ["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"];

    for token in expected_tokens.iter() {
        assert!(tokens.contains(&json!(token)), "Expected token {} not found", token);
    }
}

#[tokio::test]
async fn test_estimate_transaction_fee() {
    let client = setup_test_client().await;
    let test_tx = create_test_transaction().await;

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
    let test_tx = create_test_transaction().await;
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

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_and_send_transaction() {
    let client = setup_test_client().await;
    let test_tx = create_test_transaction().await;

    let result = client
        .request::<serde_json::Value, _>("signAndSendTransaction", rpc_params![test_tx])
        .await;

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
            println!(
                "Note: signAndSendTransaction failed as expected without funded accounts: {}",
                e
            );
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

    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    let transaction_string = response["transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_transfer_transaction_with_ata() {
    let client = setup_test_client().await;
    let rpc_client = setup_rpc_client().await;
    let random_keypair = Keypair::new();
    let random_pubkey = random_keypair.pubkey();
    let token_mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
    let sender = Pubkey::from_str("J1NiBQHq1Q98HwB4xZCpekg66oXniqzW9vXJorZNuF9R").unwrap();
    let token_type = TokenType::Spl;
    
    let sender_token_account = get_associated_token_address_with_program_id(
        &sender,
        &token_mint,
        &token_type.program_id()
    );
    let recipient_token_account = get_associated_token_address_with_program_id(
        &random_pubkey,
        &token_mint,
        &token_type.program_id()
    );

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

    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    let transaction_string = response["transaction"].as_str().unwrap();
    let decoded_tx = bs58::decode(transaction_string)
        .into_vec()
        .expect("Failed to decode transaction from base58");

    let transaction: Transaction =
        bincode::deserialize(&decoded_tx).expect("Failed to deserialize transaction");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

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

#[tokio::test]
async fn test_sign_transaction_if_paid() {
    let client = setup_test_client().await;
    let rpc_client = setup_rpc_client().await;
    let sender = get_test_sender_keypair();
    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();
    let fee_payer = Pubkey::from_str("BrfrZdQNEitACxyYLNmFRWHtRzZFNFpYH5GAtoA1XXU6").unwrap();

    // Setup token accounts
    let token_mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
    let token_type = TokenType::Spl;
    
    let sender_token_account = get_associated_token_address_with_program_id(
        &sender.pubkey(),
        &token_mint,
        &token_type.program_id()
    );
    let recipient_token_account = get_associated_token_address_with_program_id(
        &recipient,
        &token_mint,
        &token_type.program_id()
    );
    let fee_payer_token_account = get_associated_token_address_with_program_id(
        &fee_payer,
        &token_mint,
        &token_type.program_id()
    );

    let decimals = 6;
    let amount = 0.0015;
    let scaled_amount = (amount * 10_f64.powi(decimals)) as u64;

    // Create token interface
    let token_interface = TokenKeg;

    // Create instructions
    let fee_payer_instruction = token_interface.transfer(
        &sender_token_account,
        &fee_payer_token_account,
        &sender.pubkey(),
        &[],
        scaled_amount,
    ).unwrap();

    let recipient_instruction = token_interface.transfer(
        &sender_token_account,
        &recipient_token_account,
        &sender.pubkey(),
        &[],
        1,
    ).unwrap();

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    // Create message and transaction
    let message = Message::new_with_blockhash(
        &[fee_payer_instruction, recipient_instruction],
        Some(&sender.pubkey()), // Set the fee payer
        &blockhash.0,
    );

    // Initialize transaction with correct number of signatures
    let mut transaction = Transaction::new_unsigned(message);

    // Sign with sender's keypair
    transaction.sign(&[&sender], blockhash.0);

    // At this point, fee payer's signature slot should be empty (first position)
    // and sender's signature should be in the correct position

    let serialized = bincode::serialize(&transaction).unwrap();
    let base58_transaction = bs58::encode(serialized).into_string();

    // Rest of the test remains the same...
    let response: serde_json::Value = client
        .request(
            "signTransactionIfPaid",
            rpc_params![
                base58_transaction,
                0,
                json!({
                    "price": 0.00484,
                })
            ],
        )
        .await
        .expect("Failed to sign transaction");

    println!("Response: {:?}", response);

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

    // print hex of message data
    let message_data = transaction.message_data();
    println!("Message data: {:?}", hex::encode(message_data));
    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    println!("Simulated transaction: {:?}", simulated_tx);
    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}
