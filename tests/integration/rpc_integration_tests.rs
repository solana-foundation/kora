use crate::common::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use jsonrpsee::{core::client::ClientT, rpc_params};
use kora_lib::{
    token::{TokenInterface, TokenProgram},
    transaction::{TransactionUtil, VersionedTransactionOps},
};
use serde_json::json;
use solana_commitment_config::CommitmentConfig;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use solana_system_interface::{instruction::transfer, program::ID as SYSTEM_PROGRAM_ID};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

#[tokio::test]
async fn test_get_supported_tokens() {
    let client = ClientTestHelper::get_test_client().await;

    let response: serde_json::Value = client
        .request("getSupportedTokens", rpc_params![])
        .await
        .expect("Failed to get supported tokens");

    let tokens = response["tokens"].as_array().expect("Expected tokens array");
    assert!(!tokens.is_empty(), "Tokens list should not be empty");

    // Check for specific known tokens
    let expected_tokens = [&USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string()];

    for token in expected_tokens.iter() {
        assert!(tokens.contains(&json!(token)), "Expected token {token} not found");
    }
}

#[tokio::test]
async fn test_estimate_transaction_fee() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = client
        .request(
            "estimateTransactionFee",
            rpc_params![test_tx, USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string()],
        )
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
}

#[tokio::test]
async fn test_sign_transaction() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");
    let rpc_client = RPCTestHelper::get_rpc_client().await;

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
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_spl_transaction() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_spl_transaction()
        .await
        .expect("Failed to create test SPL transaction");
    let rpc_client = RPCTestHelper::get_rpc_client().await;
    let sender = SenderTestHelper::get_test_sender_keypair();

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
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let mut resolved_transaction =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(transaction.message);

    let sender_position = resolved_transaction
        .find_signer_position(&sender.pubkey())
        .expect("Sender not found in account keys");

    let signature = sender.sign_message(&resolved_transaction.transaction.message.serialize());
    resolved_transaction.transaction.signatures[sender_position] = signature;

    let simulated_tx = rpc_client
        .simulate_transaction(&resolved_transaction.transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_and_send_transaction() {
    let client = ClientTestHelper::get_test_client().await;
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let amount = 10;
    let rpc_client = RPCTestHelper::get_rpc_client().await;

    let instruction = transfer(&sender.pubkey(), &recipient, amount);

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[instruction],
        Some(&fee_payer),
        &blockhash.0,
    ));

    // Create transaction and partially sign it with sender
    let mut resolved_transaction =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

    let sender_position = resolved_transaction
        .find_signer_position(&sender.pubkey())
        .expect("Sender not found in account keys");

    let signature = sender.sign_message(&resolved_transaction.transaction.message.serialize());
    resolved_transaction.transaction.signatures[sender_position] = signature;

    // Encode transaction as base64 to send to backend
    let test_tx = resolved_transaction.encode_b64_transaction().unwrap();

    let result = client
        .request::<serde_json::Value, _>("signAndSendTransaction", rpc_params![test_tx])
        .await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed");
    let response = result.unwrap();
    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_invalid_transaction() {
    let client = ClientTestHelper::get_test_client().await;
    let invalid_tx = "invalid_base64_transaction";

    let result =
        client.request::<serde_json::Value, _>("signTransaction", rpc_params![invalid_tx]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

#[tokio::test]
async fn test_transfer_transaction() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

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
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_transfer_transaction_with_ata() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;
    let random_keypair = Keypair::new();
    let random_pubkey = random_keypair.pubkey();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let response: serde_json::Value = client
        .request(
            "transferTransaction",
            rpc_params![
                10,
                &USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string(),
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
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_liveness_is_disabled() {
    let client = ClientTestHelper::get_test_client().await;

    let response = client.request::<serde_json::Value, _>("liveness", rpc_params![]).await;
    assert!(response.is_err());
    assert!(response.err().unwrap().to_string().contains("Method not found"));
}

#[tokio::test]
async fn test_get_blockhash() {
    let client = ClientTestHelper::get_test_client().await;

    let response: serde_json::Value =
        client.request("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");
}

#[tokio::test]
async fn test_get_config() {
    let client = ClientTestHelper::get_test_client().await;

    let response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");
    assert!(response["fee_payers"].as_array().is_some(), "Expected fee_payers array in response");
    assert!(
        !response["fee_payers"].as_array().unwrap().is_empty(),
        "Expected at least one fee payer"
    );
    assert!(
        response["validation_config"].as_object().is_some(),
        "Expected validation_config in response"
    );
}

#[tokio::test]
async fn test_get_payer_signer() {
    let client = ClientTestHelper::get_test_client().await;

    let response: serde_json::Value =
        client.request("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");
    assert!(response["signer_address"].as_str().is_some(), "Expected signer_address in response");
    assert!(response["payment_address"].as_str().is_some(), "Expected payment_address in response");
}

#[tokio::test]
async fn test_sign_transaction_if_paid() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;

    // get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Setup token accounts using deterministic test USDC mint
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let sender_token_account = get_associated_token_address(&sender.pubkey(), &token_mint);
    let recipient_token_account = get_associated_token_address(&recipient, &token_mint);
    let fee_payer_token_account = get_associated_token_address(&fee_payer, &token_mint);

    let fee_amount = 100000;

    // Create instructions
    let token_interface = TokenProgram::new();
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
    let mut resolved_transaction =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

    let sender_position = resolved_transaction
        .find_signer_position(&sender.pubkey())
        .expect("Sender not found in account keys");

    let signature = sender.sign_message(&resolved_transaction.transaction.message.serialize());
    resolved_transaction.transaction.signatures[sender_position] = signature;

    // At this point, fee payer's signature slot should be empty (first position)
    // and sender's signature should be in the correct position
    let base64_transaction = resolved_transaction.encode_b64_transaction().unwrap();

    // Rest of the test remains the same...
    let response: serde_json::Value = client
        .request("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base64 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
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
    let client = ClientTestHelper::get_test_client().await;

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
}

#[tokio::test]
async fn test_sign_v0_transaction_with_valid_lookup_table() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;

    let allowed_lookup_table_address =
        LookupTableTestHelper::get_allowed_lookup_table_address().await.unwrap();

    // Create a V0 transaction using the allowed lookup table (index 0)
    let v0_transaction = TransactionTestHelper::create_v0_transaction_with_lookup(
        &allowed_lookup_table_address,
        &RecipientTestHelper::get_recipient_pubkey(),
    )
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
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
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
    let client = ClientTestHelper::get_test_client().await;

    // Create a V0 transaction using the disallowed lookup table (index 1)
    let disallowed_lookup_table_address =
        LookupTableTestHelper::get_disallowed_lookup_table_address().await.unwrap();
    let v0_transaction = TransactionTestHelper::create_v0_transaction_with_lookup(
        &disallowed_lookup_table_address,
        &LookupTableTestHelper::get_test_disallowed_address(),
    )
    .await
    .expect("Failed to create V0 transaction with disallowed lookup table");

    // Test signing the V0 transaction through Kora RPC - this should fail due to disallowed addresses
    let result = client
        .request::<serde_json::Value, _>("signTransaction", rpc_params![v0_transaction])
        .await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

#[tokio::test]

async fn test_estimate_transaction_fee_without_fee_token() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = client
        .request("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
    assert!(
        response["fee_in_token"].is_null(),
        "Expected fee_in_token to be null when not requested"
    );
}

#[tokio::test]
async fn test_estimate_transaction_fee_with_fee_token() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string();

    let response: serde_json::Value = client
        .request("estimateTransactionFee", rpc_params![test_tx, usdc_mint])
        .await
        .expect("Failed to estimate transaction fee with token");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
    assert!(
        response["fee_in_token"].as_f64().is_some(),
        "Expected fee_in_token when fee_token is provided"
    );

    let fee_in_lamports = response["fee_in_lamports"].as_u64().unwrap();
    let fee_in_token = response["fee_in_token"].as_f64().unwrap();

    // Verify the conversion makes sense
    // Mocked price DEFAULT_MOCKED_PRICE is 0.001, so 0.001 SOL per usdc
    // Fee in lamport is 10050
    // 10000 lamports -> 0.00001 SOL -> 0.00001 / 0.001 (sol per usdc) = 0.01 usdc
    // 0.01 usdc * 10^6 = 10000 usdc in base units
    assert_eq!(fee_in_lamports, 10050, "Fee in lamports should be 10050");
    assert_eq!(fee_in_token, 10050.0, "Fee in token should be 10050");
}

#[tokio::test]
async fn test_estimate_transaction_fee_with_invalid_mint() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    let result = client
        .request::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![test_tx, "invalid_mint_address"],
        )
        .await;

    assert!(result.is_err(), "Expected error for invalid mint address");
}

#[tokio::test]
async fn test_estimate_transaction_fee_without_payment_instruction() {
    let client = ClientTestHelper::get_test_client().await;
    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = client
        .request("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee with token");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");

    let fee_in_lamports = response["fee_in_lamports"].as_u64().unwrap();

    println!("fee_in_lamports: {:?}", fee_in_lamports);
    // Fee in lamport is 10000 + payment instruction fee
    assert_eq!(fee_in_lamports, 10050, "Fee in lamports should be 10000, got {}", fee_in_lamports);
}
