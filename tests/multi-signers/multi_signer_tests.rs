use crate::common::*;
use jsonrpsee::{core::client::ClientT, rpc_params};
use serde_json::json;
use solana_sdk::signature::Signer;

#[tokio::test]
async fn test_multi_signer_get_config() {
    let client = ClientTestHelper::get_test_client().await;

    let response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");

    // Check fee_payers array
    assert!(response["fee_payers"].is_array());
    assert!(response["fee_payers"].as_array().unwrap().len() == 2);
}
#[tokio::test]
async fn test_multi_signer_round_robin_behavior() {
    let client = ClientTestHelper::get_test_client().await;

    for _ in 0..6 {
        let response: serde_json::Value =
            client.request("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");

        assert!(response["blockhash"].is_string());

        // Small delay to potentially see different signers being used
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

/// Test that signer hints work correctly for maintaining consistency across RPC calls
#[tokio::test]
async fn test_signer_hint_consistency() {
    let client = ClientTestHelper::get_test_client().await;

    // First get list of available signers from config
    let config_response: serde_json::Value =
        client.request("getConfig", rpc_params![]).await.expect("Failed to get config");

    let fee_payers = config_response["fee_payers"].as_array().unwrap();
    let first_signer_pubkey = fee_payers[0].as_str().unwrap().to_string();

    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    // Call estimateTransactionFee with signer hint
    let estimate_response: serde_json::Value = client
        .request(
            "estimateTransactionFee",
            rpc_params![
                &test_tx,
                USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string(),
                &first_signer_pubkey
            ],
        )
        .await
        .expect("Failed to estimate transaction fee");

    let estimate_signer = estimate_response["signer_pubkey"].as_str().unwrap();

    // Verify the same signer was used
    assert_eq!(estimate_signer, first_signer_pubkey, "Estimate should use hinted signer");

    // Call transferTransaction with the same signer hint
    let transfer_response: serde_json::Value = client
        .request(
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

    let transfer_signer = transfer_response["signer_pubkey"].as_str().unwrap();

    // Verify the same signer was used consistently
    assert_eq!(transfer_signer, first_signer_pubkey, "Transfer should use same hinted signer");
    assert_eq!(estimate_signer, transfer_signer, "Both calls should use same signer");

    // Now call signTransaction with the same hint using the built transaction
    let built_tx = transfer_response["transaction"].as_str().unwrap();
    let sign_response: serde_json::Value = client
        .request("signTransaction", rpc_params![built_tx, &first_signer_pubkey])
        .await
        .expect("Failed to sign transaction");

    let sign_signer = sign_response["signer_pubkey"].as_str().unwrap();

    // Verify all three calls used the same signer
    assert_eq!(sign_signer, first_signer_pubkey, "Sign should use same hinted signer");
    assert_eq!(estimate_signer, sign_signer, "All calls should use same signer");
    assert_eq!(transfer_signer, sign_signer, "All calls should use same signer");
}

/// Test that without signer hints, multiple estimate calls might get different signers (round-robin)
#[tokio::test]
async fn test_round_robin_without_hints() {
    let client = ClientTestHelper::get_test_client().await;
    let mut signers_used = std::collections::HashSet::new();

    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    // Make multiple calls without signer hints to see round-robin behavior
    for _ in 0..6 {
        let estimate_response: serde_json::Value = client
            .request(
                "estimateTransactionFee",
                rpc_params![&test_tx, USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string()],
            )
            .await
            .expect("Failed to estimate transaction fee");

        let signer_pubkey = estimate_response["signer_pubkey"].as_str().unwrap();
        signers_used.insert(signer_pubkey.to_string());
    }

    // With 2 signers configured and round-robin, we should eventually see both
    assert!(!signers_used.is_empty(), "Should see at least one signer");
    assert!(signers_used.len() >= 2, "Should see at least 2 signers");
}

/// Test invalid signer hint handling
#[tokio::test]
async fn test_invalid_signer_hint() {
    let client = ClientTestHelper::get_test_client().await;
    let invalid_pubkey = "InvalidPubkey123";

    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    // Call with invalid signer hint should fail
    let result = client
        .request::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![json!({
                "transaction": test_tx,
                "signer_hint": invalid_pubkey
            })],
        )
        .await;

    assert!(result.is_err(), "Should fail with invalid signer hint");
}

/// Test nonexistent signer hint handling
#[tokio::test]
async fn test_nonexistent_signer_hint() {
    let client = ClientTestHelper::get_test_client().await;
    let nonexistent_pubkey = "11111111111111111111111111111112"; // Valid format but not in pool

    let test_tx = TransactionTestHelper::create_test_transaction()
        .await
        .expect("Failed to create test transaction");

    // Call with nonexistent signer hint should fail
    let result = client
        .request::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![json!({
                "transaction": test_tx,
                "signer_hint": nonexistent_pubkey
            })],
        )
        .await;

    assert!(result.is_err(), "Should fail with nonexistent signer hint");
}
