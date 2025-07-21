use jsonrpsee::{core::client::ClientT, rpc_params};
use kora_lib::{
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::{
        decode_b64_transaction, encode_b64_transaction, new_unsigned_versioned_transaction,
    },
};
use serde_json::json;
use solana_commitment_config::CommitmentConfig;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;
use testing_utils::*;

#[tokio::test]
async fn test_get_supported_tokens() {
    let client = get_test_client().await;

    let response: serde_json::Value = client
        .request("getSupportedTokens", rpc_params![])
        .await
        .expect("Failed to get supported tokens");

    let tokens = response["tokens"].as_array().expect("Expected tokens array");
    assert!(!tokens.is_empty(), "Tokens list should not be empty");

    // Check for specific known tokens
    let expected_tokens = [&get_test_usdc_mint_pubkey().to_string()];

    for token in expected_tokens.iter() {
        assert!(tokens.contains(&json!(token)), "Expected token {token} not found");
    }
}

#[tokio::test]
async fn test_estimate_transaction_fee() {
    let client = get_test_client().await;
    let test_tx = create_test_transaction().await.expect("Failed to create test transaction");

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
    let client = get_test_client().await;
    let test_tx = create_test_transaction().await.expect("Failed to create test transaction");
    let rpc_client = get_rpc_client().await;

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
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_spl_transaction() {
    let client = get_test_client().await;
    let test_tx =
        create_test_spl_transaction().await.expect("Failed to create test SPL transaction");
    let rpc_client = get_rpc_client().await;
    let sender = get_test_sender_keypair();

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
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let mut transaction = transaction;

    let sender_position = transaction
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == &sender.pubkey())
        .expect("Sender not found in account keys");

    let signature = sender.sign_message(&transaction.message.serialize());
    transaction.signatures[sender_position] = signature;

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_and_send_transaction() {
    let client = get_test_client().await;
    let test_tx = create_test_transaction().await.expect("Failed to create test transaction");

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
                "Note: signAndSendTransaction failed as expected without funded accounts: {e}"
            );
        }
    }
}

#[tokio::test]
async fn test_invalid_transaction() {
    let client = get_test_client().await;
    let invalid_tx = "invalid_base64_transaction";

    let result =
        client.request::<serde_json::Value, _>("signTransaction", rpc_params![invalid_tx]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

#[tokio::test]
async fn test_transfer_transaction() {
    let client = get_test_client().await;
    let rpc_client = get_rpc_client().await;

    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();

    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                1,
                SYSTEM_PROGRAM_ID.to_string(),
                sender.pubkey().to_string(),
                recipient.to_string()
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    assert!(response["transaction"].as_str().is_some(), "Expected transaction in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    let transaction_string = response["transaction"].as_str().unwrap();
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_transfer_transaction_with_ata() {
    let client = get_test_client().await;
    let rpc_client = get_rpc_client().await;
    let random_keypair = Keypair::new();
    let random_pubkey = random_keypair.pubkey();

    let sender = get_test_sender_keypair();
    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                10,
                &get_test_usdc_mint_pubkey().to_string(),
                sender.pubkey().to_string(),
                random_pubkey.to_string()
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

    assert!(response["transaction"].as_str().is_some(), "Expected signature in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    let transaction_string = response["transaction"].as_str().unwrap();
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_get_blockhash() {
    let client = get_test_client().await;

    let response: serde_json::Value =
        client.request("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");
}

#[tokio::test]
async fn test_get_config() {
    let client = get_test_client().await;

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
    let client = get_test_client().await;
    let rpc_client = get_rpc_client().await;

    // get fee payer from config
    let response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");
    let fee_payer = Pubkey::from_str(response["fee_payer"].as_str().unwrap()).unwrap();

    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();

    // Setup token accounts using deterministic test USDC mint
    let token_mint = get_test_usdc_mint_pubkey();
    let sender_token_account = get_associated_token_address(&sender.pubkey(), &token_mint);
    let recipient_token_account = get_associated_token_address(&recipient, &token_mint);
    let fee_payer_token_account = get_associated_token_address(&fee_payer, &token_mint);

    let fee_amount = 100000;

    // Create instructions
    let token_interface = TokenProgram::new(TokenType::Spl);
    let fee_payer_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &fee_payer_token_account,
            &sender.pubkey(),
            fee_amount,
        )
        .unwrap();

    let recipient_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &recipient_token_account,
            &sender.pubkey(),
            1,
        )
        .unwrap();

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    // Create message and transaction
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[fee_payer_instruction, recipient_instruction],
        Some(&fee_payer), // Set the fee payer
        &blockhash.0,
    ));

    // Initialize transaction with correct number of signatures
    let mut transaction = new_unsigned_versioned_transaction(message);

    let sender_position = transaction
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == &sender.pubkey())
        .expect("Sender not found in account keys");

    let signature = sender.sign_message(&transaction.message.serialize());
    transaction.signatures[sender_position] = signature;

    // At this point, fee payer's signature slot should be empty (first position)
    // and sender's signature should be in the correct position
    let base64_transaction = encode_b64_transaction(&transaction).unwrap();

    // Rest of the test remains the same...
    let response: serde_json::Value = client
        .request("signTransactionIfPaid", rpc_params![base64_transaction, 0])
        .await
        .expect("Failed to sign transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base64 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_fee_payer_policy_is_present() {
    let client = get_test_client().await;

    let config_response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");

    let validation_config = config_response["validation_config"]
        .as_object()
        .expect("Expected validation_config in response");

    let fee_payer_policy = validation_config["fee_payer_policy"]
        .as_object()
        .expect("Expected fee_payer_policy in validation_config");

    assert!(fee_payer_policy.contains_key("allow_sol_transfers"));
    assert!(fee_payer_policy.contains_key("allow_spl_transfers"));
    assert!(fee_payer_policy.contains_key("allow_token2022_transfers"));
    assert!(fee_payer_policy.contains_key("allow_assign"));

    assert_eq!(fee_payer_policy["allow_sol_transfers"], true);
    assert_eq!(fee_payer_policy["allow_spl_transfers"], true);
    assert_eq!(fee_payer_policy["allow_token2022_transfers"], true);
    assert_eq!(fee_payer_policy["allow_assign"], true);

    println!("Fee payer policy API integration tests passed!");
}
#[tokio::test]
async fn test_sign_v0_transaction_with_valid_lookup_table() {
    let client = get_test_client().await;
    let rpc_client = get_rpc_client().await;

    let allowed_lookup_table_address = get_allowed_lookup_table_address().await.unwrap();

    // Create a V0 transaction using the allowed lookup table (index 0)
    let v0_transaction =
        create_v0_transaction_with_lookup(&allowed_lookup_table_address, &get_recipient_pubkey())
            .await
            .expect("Failed to create V0 transaction with allowed lookup table");

    // Test signing the V0 transaction through Kora RPC - this should succeed
    let response: serde_json::Value = client
        .request("signTransaction", rpc_params![v0_transaction])
        .await
        .expect("Failed to sign V0 transaction with valid lookup table");

    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction to ensure it's valid
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction simulation failed");
}

#[tokio::test]
async fn test_sign_v0_transaction_with_invalid_lookup_table() {
    let client = get_test_client().await;

    // Create a V0 transaction using the disallowed lookup table (index 1)
    let disallowed_lookup_table_address = get_disallowed_lookup_table_address().await.unwrap();
    let v0_transaction = create_v0_transaction_with_lookup(
        &disallowed_lookup_table_address,
        &get_test_disallowed_address(),
    )
    .await
    .expect("Failed to create V0 transaction with disallowed lookup table");

    // Test signing the V0 transaction through Kora RPC - this should fail due to disallowed addresses
    let result = client
        .request::<serde_json::Value, _>("signTransaction", rpc_params![v0_transaction])
        .await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}
