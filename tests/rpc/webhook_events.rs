// Webhook Integration Tests
//
// These tests ONLY run in the webhook test phase (port 8088)
// They are ignored in regular test runs because webhooks are not enabled

use crate::common::*;
use jsonrpsee::rpc_params;
use mockito::Server;
use solana_sdk::signer::Signer;
use std::time::Duration;
use tokio::time::sleep;

/// Test that webhook is called after successful transaction signing
#[tokio::test]
#[ignore = "Only runs with webhook config (test_runner --tests webhook)"]
async fn test_webhook_called_on_sign_transaction() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .match_header("content-type", "application/json")
        .match_header("x-webhook-signature", mockito::Matcher::Any)
        .match_header("x-webhook-timestamp", mockito::Matcher::Any)
        .with_status(200)
        .with_body("ok")
        .expect(1)
        .create_async()
        .await;

    println!("Mock webhook server: {}/webhook", server.url());

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(&token_mint, &sender.pubkey(), &fee_payer, 100)
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();
    sleep(Duration::from_millis(500)).await;
    mock.assert_async().await;
}

/// Test webhook payload structure
#[tokio::test]
#[ignore = "Only runs with webhook config (test_runner --tests webhook)"]
async fn test_webhook_payload_structure() {
    let mut server = Server::new_async().await;
    
    let mock = server
        .mock("POST", "/webhook")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "event": "transaction.signed",
            "data": {
                "method": "signTransaction"
            }
        })))
        .with_status(200)
        .expect(1)
        .create_async()
        .await;

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(&token_mint, &sender.pubkey(), &fee_payer, 100)
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

/// Test all RPC methods emit webhooks
#[tokio::test]
#[ignore = "Only runs with webhook config (test_runner --tests webhook)"]
async fn test_webhook_on_all_methods() {
    let mut server = Server::new_async().await;
    
    // Expect 3 calls (sign, signAndSend, transfer)
    let mock = server
        .mock("POST", "/webhook")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "event": "transaction.signed"
        })))
        .with_status(200)
        .expect(3)
        .create_async()
        .await;

    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Test signTransaction
    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(&token_mint, &sender.pubkey(), &fee_payer, 100)
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create test transaction");

    ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![test_tx.clone()])
        .await
        .expect("Failed to sign transaction");

    // Test signAndSendTransaction
    ctx.rpc_call::<serde_json::Value, _>("signAndSendTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign and send");

    // Test transferTransaction
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

    sleep(Duration::from_secs(2)).await;
    mock.assert_async().await;
}