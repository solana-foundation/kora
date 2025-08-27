use crate::common::*;
use jsonrpsee::rpc_params;
use serde_json::json;
use solana_sdk::signature::Signer;

#[tokio::test]
async fn test_multi_signer_get_config() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();

    assert!(response["fee_payers"].is_array());
    assert!(response["fee_payers"].as_array().unwrap().len() == 2);
}

#[tokio::test]
async fn test_multi_signer_get_payer_signer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value =
        ctx.rpc_call("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");

    response.assert_success();
    assert!(response["signer_address"].as_str().is_some(), "Expected signer_address in response");
    assert!(response["payment_address"].as_str().is_some(), "Expected payment_address in response");
}

#[tokio::test]
async fn test_multi_signer_round_robin_behavior() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    for _ in 0..6 {
        let response: serde_json::Value =
            ctx.rpc_call("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");

        response.assert_success();
        assert!(response["blockhash"].is_string());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// Test that signer keys work correctly for maintaining consistency across RPC calls
#[tokio::test]
async fn test_signer_key_consistency() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    // First get list of available signers from config
    let config_response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    config_response.assert_success();
    let fee_payers = config_response["fee_payers"].as_array().unwrap();
    let first_signer_pubkey = fee_payers[0].as_str().unwrap().to_string();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    // Call estimateTransactionFee with signer key
    let estimate_response: serde_json::Value = ctx
        .rpc_call(
            "estimateTransactionFee",
            rpc_params![
                &test_tx,
                USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string(),
                &first_signer_pubkey
            ],
        )
        .await
        .expect("Failed to estimate transaction fee");

    estimate_response.assert_success();
    let estimate_signer = estimate_response["signer_pubkey"].as_str().unwrap();

    // Verify the same signer was used
    assert_eq!(estimate_signer, first_signer_pubkey, "Estimate should use signer keyed signer");

    // Call transferTransaction with the same signer key
    let transfer_response: serde_json::Value = ctx
        .rpc_call(
            "transferTransaction",
            rpc_params![
                100u64,
                "11111111111111111111111111111111", // Native SOL
                SenderTestHelper::get_test_sender_keypair().pubkey().to_string(),
                RecipientTestHelper::get_recipient_pubkey().to_string(),
                &first_signer_pubkey
            ],
        )
        .await
        .expect("Failed to create transfer transaction");

    transfer_response.assert_success();
    let transfer_signer = transfer_response["signer_pubkey"].as_str().unwrap();

    // Verify the same signer was used consistently
    assert_eq!(
        transfer_signer, first_signer_pubkey,
        "Transfer should use same signer keyed signer"
    );
    assert_eq!(estimate_signer, transfer_signer, "Both calls should use same signer");

    // Now call signTransaction with the same signer key using the built transaction
    let built_tx = transfer_response["transaction"].as_str().unwrap();
    let sign_response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![built_tx, &first_signer_pubkey])
        .await
        .expect("Failed to sign transaction");

    sign_response.assert_success();
    let sign_signer = sign_response["signer_pubkey"].as_str().unwrap();

    // Verify all three calls used the same signer
    assert_eq!(sign_signer, first_signer_pubkey, "Sign should use same signer keyed signer");
    assert_eq!(estimate_signer, sign_signer, "All calls should use same signer");
    assert_eq!(transfer_signer, sign_signer, "All calls should use same signer");
}

/// Test that without signer keys, multiple estimate calls get different signers (round-robin)
#[tokio::test]
async fn test_round_robin_without_signer_keys() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let mut signers_used = std::collections::HashSet::new();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    // Make multiple calls without signer keys to see round-robin behavior
    for _ in 0..6 {
        let estimate_response: serde_json::Value = ctx
            .rpc_call(
                "estimateTransactionFee",
                rpc_params![&test_tx, USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string()],
            )
            .await
            .expect("Failed to estimate transaction fee");

        estimate_response.assert_success();
        let signer_pubkey = estimate_response["signer_pubkey"].as_str().unwrap();
        signers_used.insert(signer_pubkey.to_string());
    }

    assert!(!signers_used.is_empty(), "Should see at least one signer");
    assert!(signers_used.len() >= 2, "Should see at least 2 signers");
}

/// Test invalid signer key handling
#[tokio::test]
async fn test_invalid_signer_key() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let invalid_pubkey = "InvalidPubkey123";

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![json!({
                "transaction": test_tx,
                "signer_key": invalid_pubkey
            })],
        )
        .await;

    assert!(result.is_err(), "Should fail with invalid signer key");
}

/// Test nonexistent signer key handling
#[tokio::test]
async fn test_nonexistent_signer_key() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let nonexistent_pubkey = "11111111111111111111111111111112"; // Valid format but not in pool

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![json!({
                "transaction": test_tx,
                "signer_key": nonexistent_pubkey
            })],
        )
        .await;

    assert!(result.is_err(), "Should fail with nonexistent signer key");
}
