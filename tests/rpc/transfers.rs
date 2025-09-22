use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::signature::{Keypair, Signer};

/// Test transferTransaction with SPL token transfer
#[tokio::test]
async fn test_transfer_transaction_spl_token_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let amount = 1_000_000; // 1 USDC

    let request_params = rpc_params![
        amount,
        token_mint.to_string(),
        sender.pubkey().to_string(),
        recipient.to_string()
    ];

    let response: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("transferTransaction", request_params).await;

    println!("DEBUG: Response: {response:?}");
    assert!(response.is_ok(), "Failed to transfer SPL token {response:?}");

    // let response: serde_json::Value = ctx
    //     .rpc_call("transferTransaction", request_params)
    //     .await
    //     .expect("Failed to transfer SPL token");

    // response.assert_success();

    // // transferTransaction returns unsigned transaction data, not a signed transaction
    // assert!(response["transaction"].as_str().is_some(), "Expected transaction in response");
    // assert!(response["message"].as_str().is_some(), "Expected message in response");
    // assert!(response["blockhash"].as_str().is_some(), "Expected blockhash in response");
}

/// Test transfer transaction with automatic ATA creation
#[tokio::test]
async fn test_transfer_transaction_with_ata_legacy() {
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
                &USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string(),
                sender.pubkey().to_string(),
                random_pubkey.to_string()
            ],
        )
        .await
        .expect("Failed to submit transfer transaction");

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
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}
