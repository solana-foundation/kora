// External Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: External system integrations and dependencies
//        - Oracle price feed integration
//        - Jito bundle API integration
//        - Address lookup table resolution
//        - External API interactions

mod jito_integration;
mod jupiter_integration;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;

use common::{harness_context, KoraSpec, TestContext};

pub async fn ctx() -> TestContext {
    harness_context(KoraSpec {
        config: "tests/src/common/fixtures/kora-test.toml",
        signers: "tests/src/common/fixtures/signers.toml",
        initialize_payments_atas: false,
    })
    .await
}
