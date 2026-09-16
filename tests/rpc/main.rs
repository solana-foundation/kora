// RPC Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: Core JSON-RPC functionality with all transaction variants
//        - Basic endpoints (getConfig, getBlockhash, etc.)
//        - Fee estimation with legacy, V0, V0+lookup, compute budget scenarios
//        - Transaction signing with all formats and conditions
//        - Transfer operations with various token types
//        - Bundle signing operations (signBundle, signAndSendBundle)
//        - Durable transaction blocking (nonce-based transactions)

mod basic_endpoints;
mod bundles;
mod compute_budget;
mod durable_transactions;
mod fee_estimation;
mod transaction_signing;
mod transfers;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;

use common::{KoraHarness, KoraSpec, TestContext, TEST_SERVER_URL_ENV};
use tokio::sync::OnceCell;

static HARNESS: OnceCell<KoraHarness> = OnceCell::const_new();

/// Kora keeps its config in process-global state, so one node per config means
/// one harness per binary.
///
/// Clients are rebuilt per test rather than shared: every `#[tokio::test]` owns
/// its runtime, and a client outliving that runtime loses its dispatch task.
///
/// `TEST_SERVER_URL` means the legacy `test_runner` already booted a validator
/// and a node; defer to it so both paths keep running. Drop with the runner.
pub async fn ctx() -> TestContext {
    if std::env::var(TEST_SERVER_URL_ENV).is_ok() {
        return TestContext::new().await.expect("Failed to create test context");
    }

    let harness = HARNESS
        .get_or_init(|| async {
            KoraHarness::start(KoraSpec {
                config: "tests/src/common/fixtures/kora-test.toml",
                signers: "tests/src/common/fixtures/signers.toml",
                initialize_payments_atas: false,
            })
            .await
            .expect("Failed to start Kora harness")
        })
        .await;

    TestContext::with_urls(harness.server_url.clone(), harness.rpc_url.clone())
        .await
        .expect("Failed to create test context")
}
