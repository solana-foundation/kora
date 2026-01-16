use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::{Keypair, Signer};

/// Test windowed limit kicks in before lifetime
/// Config: lifetime=5, windowed=4/30s
/// Expects: 4 tx succeed, 5th fails (windowed)
#[tokio::test]
async fn test_windowed_limit_before_lifetime() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // First 4 transactions should succeed (windowed limit is 4 per 30s)
    for i in 1..=4 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to sign transaction #{i}"));

        response.assert_success();
    }

    // 5th transaction should fail (exceeds windowed limit of 4 per 30s)
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for exceeding windowed limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test transaction + instruction rules - combined enforcement
/// Config: max 3 CreateAccount instructions (lifetime), max 4 tx/30s (windowed), max 5 tx (lifetime)
/// Note: Instruction limits only count instructions where Kora is the payer (subsidized account creations)
#[tokio::test]
async fn test_transaction_and_instruction_rules_combined() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();

    // Use up instruction limit (3 CreateAccounts where Kora pays)
    for _ in 1..=3 {
        let new_account = Keypair::new();
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_system_create_account(
                &FeePayerTestHelper::get_fee_payer_pubkey(), // Kora pays for account creation
                &new_account.pubkey(),
                1000000,
                0,
                &sender.pubkey(),
            )
            .with_signer(&sender)
            .with_signer(&new_account)
            .build()
            .await
            .expect("Failed to build transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .expect("Failed to sign transaction");

        response.assert_success();
    }

    // 4th CreateAccount should fail due to instruction limit
    // (even though transaction limits would allow more)
    let new_account = Keypair::new();
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(), // Kora pays for account creation
            &new_account.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_signer(&sender)
        .with_signer(&new_account)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for exceeding instruction limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test first rule violation - transaction denied on first exceeded rule
/// Config: windowed=4/30s checked first, then lifetime=5
#[tokio::test]
async fn test_first_rule_violation_denies_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Use up windowed limit (4 per 30s)
    for i in 1..=4 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to sign transaction #{i}"));

        response.assert_success();
    }

    // 5th should fail immediately on windowed limit check
    // (even though lifetime limit of 5 hasn't been reached)
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for first rule violation (windowed limit)");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test all rules pass - transaction allowed when all rules satisfied
#[tokio::test]
async fn test_all_rules_pass_transaction_allowed() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // First transaction should pass all rules:
    // - Transaction lifetime limit: 1/5 ✓
    // - Transaction windowed limit: 1/4 ✓
    // - Instruction limit: N/A (no CreateAccount) ✓
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction");

    let response: serde_json::Value = ctx
        .rpc_call(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await
        .expect("Failed to sign transaction when all rules satisfied");

    response.assert_success();
    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
}
