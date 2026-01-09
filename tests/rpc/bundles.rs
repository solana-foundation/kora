// Bundle Integration Tests
//
// Tests for signBundle and signAndSendBundle RPC methods

use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::signature::Signer;
use std::str::FromStr;

// **************************************************************************************
// signBundle tests
// **************************************************************************************

/// Test signing a single legacy transaction bundle
#[tokio::test]
async fn test_sign_bundle_single_legacy_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create a single transaction
    let tx1 = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create transaction");

    let transactions = vec![tx1];
    let response: serde_json::Value =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await.expect("Failed to sign bundle");

    response.assert_success();

    assert!(
        response["signed_transactions"].is_array(),
        "Expected signed_transactions array in response"
    );
    assert_eq!(
        response["signed_transactions"].as_array().unwrap().len(),
        1,
        "Expected 1 signed transaction"
    );
    assert!(response["signer_pubkey"].as_str().is_some(), "Expected signer_pubkey in response");
}

/// Test signing multiple legacy transactions in a bundle
#[tokio::test]
async fn test_sign_bundle_multiple_legacy_transactions() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create 3 transactions for the bundle
    let mut transactions = Vec::new();
    for i in 0..3 {
        let tx = ctx
            .transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_spl_transfer(
                &token_mint,
                &sender.pubkey(),
                &fee_payer,
                tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            )
            .with_transfer(&sender.pubkey(), &recipient, 10 + i)
            .build()
            .await
            .expect("Failed to create transaction");
        transactions.push(tx);
    }

    let response: serde_json::Value =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await.expect("Failed to sign bundle");

    response.assert_success();

    let signed_txs = response["signed_transactions"].as_array().unwrap();
    assert_eq!(signed_txs.len(), 3, "Expected 3 signed transactions");
}

/// Test signing max size bundle (5 transactions)
#[tokio::test]
async fn test_sign_bundle_max_size() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create 5 transactions (max allowed)
    let mut transactions = Vec::new();
    for i in 0..5 {
        let tx = ctx
            .transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_spl_transfer(
                &token_mint,
                &sender.pubkey(),
                &fee_payer,
                tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            )
            .with_transfer(&sender.pubkey(), &recipient, 10 + i)
            .build()
            .await
            .expect("Failed to create transaction");
        transactions.push(tx);
    }

    let response: serde_json::Value = ctx
        .rpc_call("signBundle", rpc_params![transactions])
        .await
        .expect("Failed to sign bundle with 5 transactions");

    response.assert_success();

    let signed_txs = response["signed_transactions"].as_array().unwrap();
    assert_eq!(signed_txs.len(), 5, "Expected 5 signed transactions");
}

/// Test signing V0 transactions in bundle
#[tokio::test]
async fn test_sign_bundle_v0_transactions() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create V0 transaction
    let tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction");

    let transactions = vec![tx];
    let response: serde_json::Value = ctx
        .rpc_call("signBundle", rpc_params![transactions])
        .await
        .expect("Failed to sign V0 bundle");

    response.assert_success();
    assert_eq!(response["signed_transactions"].as_array().unwrap().len(), 1);
}

/// Test empty bundle returns error
#[tokio::test]
async fn test_sign_bundle_empty_error() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let transactions: Vec<String> = vec![];
    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await;

    assert!(result.is_err(), "Expected error for empty bundle");
}

/// Test bundle too large (>5 transactions) returns error
#[tokio::test]
async fn test_sign_bundle_too_large_error() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create 6 transactions (exceeds max)
    let mut transactions = Vec::new();
    for i in 0..6 {
        let tx = ctx
            .transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_spl_transfer(
                &token_mint,
                &sender.pubkey(),
                &fee_payer,
                tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            )
            .with_transfer(&sender.pubkey(), &recipient, 10 + i)
            .build()
            .await
            .expect("Failed to create transaction");
        transactions.push(tx);
    }

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await;

    assert!(result.is_err(), "Expected error for bundle > 5 transactions");
}

/// Test invalid transaction in bundle returns error
#[tokio::test]
async fn test_sign_bundle_invalid_transaction_error() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let transactions = vec!["invalid_base64_transaction".to_string()];
    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

/// Test response structure has all required fields
#[tokio::test]
async fn test_sign_bundle_response_structure() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create transaction");

    let transactions = vec![tx];
    let response: serde_json::Value =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await.expect("Failed to sign bundle");

    assert!(response.get("signed_transactions").is_some(), "Missing signed_transactions field");
    assert!(response.get("signer_pubkey").is_some(), "Missing signer_pubkey field");

    // Validate signed_transactions is array of strings
    let signed_txs = response["signed_transactions"].as_array().unwrap();
    for tx in signed_txs {
        assert!(tx.is_string(), "signed_transactions should contain strings");
        // Verify it's valid base64 and can be decoded
        let decoded = TransactionUtil::decode_b64_transaction(tx.as_str().unwrap());
        assert!(decoded.is_ok(), "signed_transaction should be valid base64");
    }

    // Validate signer_pubkey is a valid pubkey
    let signer_pubkey = response["signer_pubkey"].as_str().unwrap();
    let pubkey_result = solana_sdk::pubkey::Pubkey::from_str(signer_pubkey);
    assert!(pubkey_result.is_ok(), "signer_pubkey should be valid pubkey");
}

// **************************************************************************************
// signAndSendBundle tests
// **************************************************************************************

/// Test sign and send bundle to Jito
#[tokio::test]
async fn test_sign_and_send_bundle_success() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create transaction");

    let transactions = vec![tx];
    let response: serde_json::Value = ctx
        .rpc_call("signAndSendBundle", rpc_params![transactions])
        .await
        .expect("Failed to sign and send bundle");

    response.assert_success();

    assert!(response["signed_transactions"].is_array(), "Expected signed_transactions array");
    assert!(response["signer_pubkey"].as_str().is_some(), "Expected signer_pubkey");
    assert!(response["bundle_uuid"].as_str().is_some(), "Expected bundle_uuid from Jito");
}

/// Test signAndSendBundle empty bundle error
#[tokio::test]
async fn test_sign_and_send_bundle_empty_error() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let transactions: Vec<String> = vec![];
    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendBundle", rpc_params![transactions]).await;

    assert!(result.is_err(), "Expected error for empty bundle");
}

/// Test signAndSendBundle too large error
#[tokio::test]
async fn test_sign_and_send_bundle_too_large_error() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create 6 transactions (exceeds max)
    let mut transactions = Vec::new();
    for i in 0..6 {
        let tx = ctx
            .transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_spl_transfer(
                &token_mint,
                &sender.pubkey(),
                &fee_payer,
                tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            )
            .with_transfer(&sender.pubkey(), &recipient, 10 + i)
            .build()
            .await
            .expect("Failed to create transaction");
        transactions.push(tx);
    }

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendBundle", rpc_params![transactions]).await;

    assert!(result.is_err(), "Expected error for bundle > 5 transactions");
}
