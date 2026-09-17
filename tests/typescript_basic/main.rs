#[path = "../src/common/mod.rs"]
mod common;

#[path = "../typescript/runner.rs"]
mod runner;

use common::KoraSpec;

#[tokio::test]
async fn typescript_integration_suite() {
    runner::run_suite(
        KoraSpec {
            config: "tests/src/common/fixtures/kora-test.toml",
            signers: "tests/src/common/fixtures/signers.toml",
            initialize_payments_atas: false,
        },
        &[],
    )
    .await
}
