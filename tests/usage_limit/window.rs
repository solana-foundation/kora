use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::{Keypair, Signer};
use std::future::Future;

const WINDOW_SECS: u64 = 30;
const WINDOW_ATTEMPTS: u32 = 3;
const WINDOWED_MAX: u32 = 4;

/// Windowed limits count per fixed time bucket (`unix_time / window_seconds`),
/// so the counter resets at every multiple of the window.
fn current_window_bucket() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
        / WINDOW_SECS
}

/// Returns what `body` produced, but only once it ran without a window boundary
/// elapsing: a reset mid-body lifts the very limit under test, so the outcome
/// proves nothing and has to be retried. `body` must provision its own wallets,
/// since a retry needs counters that start from zero.
pub async fn within_one_window<F, Fut, T>(what: &str, body: F) -> T
where
    F: Fn() -> Fut,
    Fut: Future<Output = T>,
{
    for _ in 0..WINDOW_ATTEMPTS {
        let start_bucket = current_window_bucket();
        let outcome = body().await;
        if current_window_bucket() == start_bucket {
            return outcome;
        }
    }

    panic!(
        "{what} never completed inside one {WINDOW_SECS}s window across \
        {WINDOW_ATTEMPTS} attempts; environment too slow to verify windowed limits"
    )
}

pub async fn send_transfer(
    ctx: &TestContext,
    sender: &Keypair,
    user_id: &str,
) -> anyhow::Result<serde_json::Value> {
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &RecipientTestHelper::get_recipient_pubkey(), 1000)
        .with_signer(sender)
        .build()
        .await
        .expect("Failed to build transaction");

    ctx.rpc_call("signAndSendTransaction", rpc_params![tx_b64, None::<String>, false, user_id])
        .await
}

/// Sends the full windowed allowance, asserting every send is accepted.
pub async fn fill_windowed_limit(ctx: &TestContext, sender: &Keypair, user_id: &str) {
    for i in 1..=WINDOWED_MAX {
        send_transfer(ctx, sender, user_id)
            .await
            .unwrap_or_else(|e| panic!("Failed to sign transaction #{i}: {e}"))
            .assert_success();
    }
}

/// Fills a fresh wallet's windowed allowance and returns the outcome of the send
/// past it, for the caller to assert on.
pub async fn transfer_past_windowed_limit(ctx: &TestContext) -> anyhow::Result<serde_json::Value> {
    within_one_window("five sends", || async {
        let sender = create_funded_wallet(ctx).await;
        let user_id = sender.pubkey().to_string();

        fill_windowed_limit(ctx, &sender, &user_id).await;
        send_transfer(ctx, &sender, &user_id).await
    })
    .await
}
