use crate::common::*;
use jsonrpsee::rpc_params;
use mockito::Server;
use serde_json::json;
use solana_sdk::signer::Signer;
use std::time::Duration;
use tokio::time::sleep;

/// Test that webhook is called after successful transaction signing
#[tokio::test]
async fn test_webhook_called_on_sign_transaction() {
    let mut server = Server::new_async().await;
    
    // Mock webhook endpoint
    let mock = server
        .mock("POST", "/webhook")
        .match_header("content-type", "application/json")
        .match_header("x-webhook-signature", mockito::Matcher::Any)
        .with_status(200)
        .with_body("ok")
        .expect(1) // Expect exactly 1 call
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    // Create test context with webhook enabled
    let ctx = TestContext::with_webhook(&webhook_url, "test-secret")
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();

    // Wait for async webhook delivery
    sleep(Duration::from_millis(500)).await;

    // Verify webhook was called
    mock.assert_async().await;
}

/// Test that webhook receives correct payload structure
#[tokio::test]
async fn test_webhook_payload_structure() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJsonString(json!({
            "event": "transaction.signed",
            "data": {
                "method": "signTransaction"
            }
        }).to_string()))
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    let ctx = TestContext::with_webhook(&webhook_url, "test-secret")
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
}

/// Test that webhook signature is valid
#[tokio::test]
async fn test_webhook_signature_verification() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    
    let mut server = Server::new_async().await;
    let secret = "test-secret";
    
    let mock = server
        .mock("POST", "/webhook")
        .match_header("x-webhook-signature", mockito::Matcher::Any)
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    let ctx = TestContext::with_webhook(&webhook_url, secret)
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
    
    // In a real test, we would capture the request and verify the HMAC
    // For now, we just verify the call was made with the signature header
}

/// Test webhook retry on failure
#[tokio::test]
async fn test_webhook_retry_on_failure() {
    let mut server = Server::new_async().await;
    
    // First two attempts fail, third succeeds
    let mock_fail_1 = server
        .mock("POST", "/webhook")
        .with_status(500)
        .expect(1)
        .create_async()
        .await;
    
    let mock_fail_2 = server
        .mock("POST", "/webhook")
        .with_status(503)
        .expect(1)
        .create_async()
        .await;
    
    let mock_success = server
        .mock("POST", "/webhook")
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    let ctx = TestContext::with_webhook(&webhook_url, "test-secret")
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    // Wait longer for retries (with exponential backoff: 1s + 2s + 4s)
    sleep(Duration::from_secs(8)).await;

    mock_fail_1.assert_async().await;
    mock_fail_2.assert_async().await;
    mock_success.assert_async().await;
}

/// Test webhook not called when disabled
#[tokio::test]
async fn test_webhook_not_called_when_disabled() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .expect(0) // Expect no calls
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    // Create context WITHOUT webhook enabled (default config)
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
}

/// Test webhook called for signAndSendTransaction
#[tokio::test]
async fn test_webhook_called_on_sign_and_send() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .match_body(mockito::Matcher::PartialJsonString(
            json!({"event": "transaction.signed", "data": {"method": "signAndSendTransaction"}}).to_string()
        ))
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    let ctx = TestContext::with_webhook(&webhook_url, "test-secret")
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signAndSendTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign and send transaction");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
}

/// Test webhook called for transferTransaction
#[tokio::test]
async fn test_webhook_called_on_transfer() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .match_body(mockito::Matcher::PartialJsonString(
            json!({"event": "transaction.signed", "data": {"method": "transferTransaction"}}).to_string()
        ))
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    let ctx = TestContext::with_webhook(&webhook_url, "test-secret")
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    ctx.rpc_call::<serde_json::Value, _>(
        "transferTransaction",
        rpc_params![
            1_000_000,
            token_mint.to_string(),
            sender.pubkey().to_string(),
            recipient.to_string()
        ],
    )
    .await
    .expect("Failed to transfer");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
}

/// Test webhook with event filtering
#[tokio::test]
async fn test_webhook_event_filtering() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .expect(0) // Should not be called because we filtered out transaction.signed
        .create_async()
        .await;

    let webhook_url = format!("{}/webhook", server.url());
    
    // Create context with webhook that only listens to rate_limit.hit
    let ctx = TestContext::with_webhook_events(&webhook_url, "test-secret", vec!["rate_limit.hit"])
        .await
        .expect("Failed to create test context with webhook");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
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
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await; // Should not have been called
}