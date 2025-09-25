use std::str::FromStr;

use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

// **************************************************************************************
// Token 2022 Transfer Tests
// **************************************************************************************

/// Test transferTransaction with Token 2022 transfer
#[tokio::test]
async fn test_transfer_transaction_token_2022_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();
    let amount = 1_000_000; // 1 USDC

    let request_params = rpc_params![
        amount,
        token_mint_2022.to_string(),
        sender.pubkey().to_string(),
        recipient.to_string()
    ];

    let response: serde_json::Value = ctx
        .rpc_call("transferTransaction", request_params)
        .await
        .expect("Failed to transfer Token 2022");

    response.assert_success();

    // transferTransaction returns unsigned transaction data, not a signed transaction
    assert!(response["transaction"].as_str().is_some(), "Expected transaction in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");
}

/// Test Token 2022 transfer transaction with automatic ATA creation
#[tokio::test]
async fn test_transfer_transaction_token_2022_with_ata_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();
    let random_keypair = Keypair::new();
    let random_pubkey = random_keypair.pubkey();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let response: serde_json::Value = ctx
        .rpc_call(
            "transferTransaction",
            rpc_params![
                10,
                &USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey().to_string(),
                sender.pubkey().to_string(),
                random_pubkey.to_string()
            ],
        )
        .await
        .expect("Failed to submit Token 2022 transfer transaction");

    response.assert_success();
    assert!(response["transaction"].as_str().is_some(), "Expected transaction in response");
    assert!(response["message"].as_str().is_some(), "Expected message in response");
    assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");

    let transaction_string = response["transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate Token 2022 transaction");

    assert!(simulated_tx.value.err.is_none(), "Token 2022 transaction simulation failed");
}

// **************************************************************************************
// Token 2022 Sign Transaction Tests
// **************************************************************************************

#[tokio::test]
async fn test_sign_token_2022_transaction_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create Token 2022 test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign Token 2022 transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate Token 2022 transaction");

    assert!(simulated_tx.value.err.is_none(), "Token 2022 transaction simulation failed");
}

#[tokio::test]
async fn test_sign_token_2022_transaction_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let test_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create V0 Token 2022 test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 Token 2022 transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 Token 2022 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 Token 2022 transaction simulation failed");
}

#[tokio::test]
async fn test_sign_token_2022_transaction_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    // Use the transaction lookup table which contains the mint address and the spl token program
    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create V0 Token 2022 test transaction with lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 Token 2022 transaction with lookup table");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 Token 2022 transaction with lookup table");

    assert!(
        simulated_tx.value.err.is_none(),
        "V0 Token 2022 transaction with lookup table simulation failed"
    );
}

// **************************************************************************************
// Token 2022 Sign and Send Transaction Tests
// **************************************************************************************

#[tokio::test]
async fn test_sign_and_send_token_2022_transaction_legacy() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create signed Token 2022 test transaction");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed for Token 2022");
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_and_send_token_2022_transaction_v0() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create V0 Token 2022 test transaction");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed for V0 Token 2022");
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_and_send_token_2022_transaction_v0_with_lookup() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    // Use the transaction lookup table which contains the mint address and the spl token program used for ATA derivation
    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 10, 6)
        .build()
        .await
        .expect("Failed to create V0 Token 2022 test transaction with lookup table");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(
        result.is_ok(),
        "Expected signAndSendTransaction to succeed for V0 Token 2022 with lookup"
    );
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

// **************************************************************************************
// Token 2022 Sign Transaction If Paid Tests
// **************************************************************************************

/// Test Token 2022 sign transaction if paid with fee payer pool logic
#[tokio::test]
async fn test_sign_token_2022_transaction_if_paid_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();
    let fee_amount = 100000;

    // Use transaction builder with proper signing and automatic ATA derivation
    let base64_transaction = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(
            &token_mint_2022,
            &sender.pubkey(),
            &fee_payer,
            fee_amount,
            6,
        )
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 1, 6)
        .build()
        .await
        .expect("Failed to create signed Token 2022 transaction");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign Token 2022 transaction");

    response.assert_success();
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
        .expect("Failed to simulate Token 2022 transaction");

    assert!(simulated_tx.value.err.is_none(), "Token 2022 transaction simulation failed");
}

/// Test Token 2022 sign transaction if paid with V0 transaction
#[tokio::test]
async fn test_sign_token_2022_transaction_if_paid_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let fee_amount = 100000;

    let base64_transaction = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(
            &token_mint_2022,
            &sender.pubkey(),
            &fee_payer,
            fee_amount,
            6,
        )
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 1, 6)
        .build()
        .await
        .expect("Failed to create V0 signed Token 2022 transaction");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 Token 2022 transaction");

    response.assert_success();
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
        .expect("Failed to simulate V0 Token 2022 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 Token 2022 transaction simulation failed");
}

/// Test Token 2022 sign transaction if paid with V0 transaction and lookup table
#[tokio::test]
async fn test_sign_token_2022_transaction_if_paid_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let fee_amount = 100000;

    // Use the transaction lookup table which contains the mint address and the spl token program
    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    // Use V0 transaction builder with lookup table and proper signing
    let base64_transaction = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked(
            &token_mint_2022,
            &sender.pubkey(),
            &fee_payer,
            fee_amount,
            6,
        )
        .with_spl_token_2022_transfer_checked(&token_mint_2022, &sender.pubkey(), &recipient, 1, 6)
        .build()
        .await
        .expect("Failed to create V0 signed Token 2022 transaction with lookup table");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 Token 2022 transaction with lookup table");

    response.assert_success();
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
        .expect("Failed to simulate V0 Token 2022 transaction with lookup table");

    assert!(
        simulated_tx.value.err.is_none(),
        "V0 Token 2022 transaction with lookup table simulation failed"
    );
}
