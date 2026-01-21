use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::Signer;

/// Test transaction limit enforcement - 4 succeed (windowed limit), 5th fails
/// Config: windowed=4/30s, lifetime=5
/// The windowed limit kicks in before lifetime limit
#[tokio::test]
async fn test_transaction_limit_enforcement() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let user_id = "test-user-tx-limit";

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
                rpc_params![tx_b64.clone(), None::<String>, false, user_id.to_string()],
            )
            .await
            .unwrap_or_else(|e| {
                eprintln!("RPC error for transaction #{i}: {:?}", e);
                panic!("Failed to sign transaction #{i}: {}", e);
            });

        response.assert_success();
        assert!(
            response["signature"].as_str().is_some(),
            "Expected signature in response for transaction #{i}"
        );
    }

    print!("Sending 5th transaction");
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
            rpc_params![tx_b64.clone(), None::<String>, false, user_id.to_string()],
        )
        .await;

    let err = result.expect_err("Expected error for 5th transaction exceeding windowed limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test transaction lifetime limit - allow N transactions, deny N+1
/// Config: lifetime=5
/// This test waits for windowed limit to reset to test lifetime limit specifically
#[tokio::test]
async fn test_transaction_lifetime_limit() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Send first 4 transactions (windowed limit)
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

    // Wait for windowed limit to reset (30 seconds + buffer)
    tokio::time::sleep(tokio::time::Duration::from_secs(31)).await;

    // Send 5th transaction - should succeed (within lifetime limit of 5)
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
        .expect("Failed to sign 5th transaction");

    response.assert_success();

    // 6th transaction should fail (exceeds lifetime limit of 5)
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

    let err = result.expect_err("Expected error for 6th transaction exceeding lifetime limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test transaction time-windowed limit - counter resets after window expires
/// Config: windowed=4/30s, lifetime=5
/// Note: This test takes ~31 seconds due to window reset wait
/// After window reset, we can only send 1 more tx (5th) because lifetime limit is 5
#[tokio::test]
async fn test_transaction_time_windowed_limit() {
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

    // 5th transaction should fail (exceeds windowed limit of 4)
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

    let err = result.expect_err("Expected error for 5th transaction exceeding windowed limit");
    err.assert_contains_message("Usage limit exceeded");

    // Wait 31 seconds for window to reset
    tokio::time::sleep(tokio::time::Duration::from_secs(31)).await;

    // After window reset, 5th transaction should succeed
    // (windowed counter reset to 0, lifetime at 4/5 - room for 1 more)
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction after window reset");

    let response: serde_json::Value = ctx
        .rpc_call(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await
        .expect("Failed to sign 5th transaction after window reset");

    response.assert_success();

    // 6th transaction should fail (lifetime limit of 5 reached)
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

    let err = result.expect_err("Expected error for 6th transaction exceeding lifetime limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test independent wallet limits - each wallet has separate counter
/// Config: windowed=4/30s - each wallet gets its own 4-tx limit
#[tokio::test]
async fn test_independent_wallet_limits() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender1 = create_funded_wallet(&ctx).await;
    let sender2 = create_funded_wallet(&ctx).await;
    let user_id1 = sender1.pubkey().to_string();
    let user_id2 = sender2.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Use up wallet1's windowed limit (4 transactions per 30s)
    for _ in 1..=4 {
        let tx1_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender1.pubkey(), &recipient, 1000)
            .with_signer(&sender1)
            .build()
            .await
            .expect("Failed to build transaction for sender1");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx1_b64, None::<String>, false, user_id1.clone()],
            )
            .await
            .expect("Failed to sign transaction for sender1");

        response.assert_success();
    }

    // Wallet1 should now be denied (hit windowed limit)
    let tx1_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender1.pubkey(), &recipient, 1000)
        .with_signer(&sender1)
        .build()
        .await
        .expect("Failed to build transaction for sender1");

    let result1 = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx1_b64, None::<String>, false, user_id1.clone()],
        )
        .await;

    let err = result1.expect_err("Expected error for sender1 exceeding limit");
    err.assert_contains_message("Usage limit exceeded");

    // Wallet2 should still be able to make transactions (independent limit)
    // Each wallet has its own windowed counter
    for _ in 1..=4 {
        let tx2_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender2.pubkey(), &recipient, 1000)
            .with_signer(&sender2)
            .build()
            .await
            .expect("Failed to build transaction for sender2");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx2_b64, None::<String>, false, user_id2.clone()],
            )
            .await
            .expect("Failed to sign transaction for sender2");

        response.assert_success();
    }
}
