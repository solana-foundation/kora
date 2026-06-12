use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::{Keypair, Signer};

const WINDOW_SECS: u64 = 30;
const WINDOW_ATTEMPTS: u32 = 3;

/// Windowed limits count per fixed time bucket (`unix_time / window_seconds`),
/// so the counter resets at every multiple of the window.
fn current_window_bucket() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
        / WINDOW_SECS
}

/// Sends four transfers (filling the 4-per-30s windowed limit) and a fifth
/// expected to exceed it. Returns None when the sends straddled a window
/// boundary — the counter legitimately reset mid-test, so the caller must
/// retry with a fresh wallet instead of asserting anything.
async fn attempt_windowed_overflow(
    ctx: &TestContext,
    sender: &Keypair,
    user_id: &str,
) -> Option<anyhow::Result<serde_json::Value>> {
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let start_bucket = current_window_bucket();

    for i in 1..=4 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(sender)
            .build()
            .await
            .expect("Failed to build transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.to_string()],
            )
            .await
            .unwrap_or_else(|e| panic!("Failed to sign transaction #{i}: {e}"));

        response.assert_success();
        assert!(
            response["signature"].as_str().is_some(),
            "Expected signature in response for transaction #{i}"
        );
    }

    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(sender)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.to_string()],
        )
        .await;

    if current_window_bucket() != start_bucket {
        return None;
    }
    Some(result)
}

/// Test transaction limit enforcement - 4 succeed (windowed limit), 5th fails
/// Config: windowed=4/30s, lifetime=5
/// The windowed limit kicks in before lifetime limit
#[tokio::test]
async fn test_transaction_limit_enforcement() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    for attempt in 0..WINDOW_ATTEMPTS {
        let sender = create_funded_wallet(&ctx).await;
        let user_id = format!("test-user-tx-limit-{attempt}");

        match attempt_windowed_overflow(&ctx, &sender, &user_id).await {
            None => continue,
            Some(Err(err)) => {
                err.assert_contains_message("Usage limit exceeded");
                return;
            }
            Some(Ok(response)) => {
                panic!("Expected 5th transaction to exceed windowed limit, got: {response}")
            }
        }
    }

    panic!(
        "five sends never completed inside the {WINDOW_SECS}s window across \
        {WINDOW_ATTEMPTS} attempts; environment too slow to verify windowed limits"
    );
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
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let mut verified = None;
    for _ in 0..WINDOW_ATTEMPTS {
        let sender = create_funded_wallet(&ctx).await;
        let user_id = sender.pubkey().to_string();

        match attempt_windowed_overflow(&ctx, &sender, &user_id).await {
            None => continue,
            Some(Err(err)) => {
                err.assert_contains_message("Usage limit exceeded");
                verified = Some((sender, user_id));
                break;
            }
            Some(Ok(response)) => {
                panic!("Expected 5th transaction to exceed windowed limit, got: {response}")
            }
        }
    }

    let Some((sender, user_id)) = verified else {
        panic!(
            "five sends never completed inside the {WINDOW_SECS}s window across \
            {WINDOW_ATTEMPTS} attempts; environment too slow to verify windowed limits"
        );
    };

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

/// Test independent wallet limits - each user_id has its own windowed counter
/// Config: windowed=4/30s
#[tokio::test]
async fn test_independent_wallet_limits() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    for _ in 0..WINDOW_ATTEMPTS {
        let start_bucket = current_window_bucket();
        let sender1 = create_funded_wallet(&ctx).await;
        let sender2 = create_funded_wallet(&ctx).await;
        let user_id1 = sender1.pubkey().to_string();
        let user_id2 = sender2.pubkey().to_string();

        // Exhaust wallet1's windowed limit and capture the over-limit outcome
        let Some(result1) = attempt_windowed_overflow(&ctx, &sender1, &user_id1).await else {
            continue;
        };

        // Wallet2 must still be allowed: counters are independent per user
        let mut responses2 = Vec::new();
        for i in 1..=4 {
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
                .unwrap_or_else(|e| panic!("Failed to sign transaction #{i} for sender2: {e}"));
            responses2.push(response);
        }

        // Only meaningful if everything stayed in one window bucket: otherwise
        // wallet2 would have been allowed even with a shared counter
        if current_window_bucket() != start_bucket {
            continue;
        }

        let err = result1.expect_err("Expected error for sender1 exceeding limit");
        err.assert_contains_message("Usage limit exceeded");
        for response in responses2 {
            response.assert_success();
        }
        return;
    }

    panic!(
        "both wallets never completed inside one {WINDOW_SECS}s window across \
        {WINDOW_ATTEMPTS} attempts; environment too slow to verify windowed limits"
    );
}
