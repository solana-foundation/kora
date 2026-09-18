use crate::{
    common::*,
    window::{fill_windowed_limit, send_transfer, transfer_past_windowed_limit, within_one_window},
};
use solana_sdk::signature::Signer;

/// Test transaction limit enforcement - 4 succeed (windowed limit), 5th fails
/// Config: windowed=4/30s, lifetime=5
/// The windowed limit kicks in before lifetime limit
#[tokio::test]
async fn test_transaction_limit_enforcement() {
    let ctx = crate::ctx().await;

    let err = transfer_past_windowed_limit(&ctx)
        .await
        .expect_err("Expected 5th transaction to exceed windowed limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test transaction lifetime limit - allow N transactions, deny N+1
/// Config: lifetime=5
/// This test waits for windowed limit to reset to test lifetime limit specifically
#[tokio::test]
async fn test_transaction_lifetime_limit() {
    let ctx = crate::ctx().await;

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();

    fill_windowed_limit(&ctx, &sender, &user_id).await;

    // Wait for the windowed limit to reset (30s window plus buffer).
    tokio::time::sleep(tokio::time::Duration::from_secs(31)).await;

    send_transfer(&ctx, &sender, &user_id)
        .await
        .expect("Failed to sign 5th transaction")
        .assert_success();

    // 6th transaction should fail (exceeds lifetime limit of 5)
    let err = send_transfer(&ctx, &sender, &user_id)
        .await
        .expect_err("Expected error for 6th transaction exceeding lifetime limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test transaction time-windowed limit - counter resets after window expires
/// Config: windowed=4/30s, lifetime=5
/// Note: This test takes ~31 seconds due to window reset wait
/// After window reset, we can only send 1 more tx (5th) because lifetime limit is 5
#[tokio::test]
async fn test_transaction_time_windowed_limit() {
    let ctx = crate::ctx().await;

    let (sender, user_id, result) = within_one_window("five sends", || async {
        let sender = create_funded_wallet(&ctx).await;
        let user_id = sender.pubkey().to_string();

        fill_windowed_limit(&ctx, &sender, &user_id).await;
        let result = send_transfer(&ctx, &sender, &user_id).await;
        (sender, user_id, result)
    })
    .await;

    let err = result.expect_err("Expected 5th transaction to exceed windowed limit");
    err.assert_contains_message("Usage limit exceeded");

    // Wait 31 seconds for window to reset
    tokio::time::sleep(tokio::time::Duration::from_secs(31)).await;

    // After window reset, 5th transaction should succeed
    // (windowed counter reset to 0, lifetime at 4/5 - room for 1 more)
    send_transfer(&ctx, &sender, &user_id)
        .await
        .expect("Failed to sign 5th transaction after window reset")
        .assert_success();

    // 6th transaction should fail (lifetime limit of 5 reached)
    let err = send_transfer(&ctx, &sender, &user_id)
        .await
        .expect_err("Expected error for 6th transaction exceeding lifetime limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test independent wallet limits - each user_id has its own windowed counter
/// Config: windowed=4/30s
#[tokio::test]
async fn test_independent_wallet_limits() {
    let ctx = crate::ctx().await;

    let (over_limit, allowed) =
        within_one_window("two wallets' sends", || async {
            let sender1 = create_funded_wallet(&ctx).await;
            let sender2 = create_funded_wallet(&ctx).await;
            let user_id1 = sender1.pubkey().to_string();
            let user_id2 = sender2.pubkey().to_string();

            fill_windowed_limit(&ctx, &sender1, &user_id1).await;
            let over_limit = send_transfer(&ctx, &sender1, &user_id1).await;

            // Wallet2 must still be allowed: counters are independent per user
            let mut allowed = Vec::new();
            for i in 1..=4 {
                allowed.push(send_transfer(&ctx, &sender2, &user_id2).await.unwrap_or_else(|e| {
                    panic!("Failed to sign transaction #{i} for sender2: {e}")
                }));
            }

            (over_limit, allowed)
        })
        .await;

    let err = over_limit.expect_err("Expected error for sender1 exceeding limit");
    err.assert_contains_message("Usage limit exceeded");
    for response in allowed {
        response.assert_success();
    }
}
