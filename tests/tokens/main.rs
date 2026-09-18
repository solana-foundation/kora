// Token Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: Token-specific functionality and integrations
//        - SPL token operations and transfers
//        - Token2022 features and validation
//        - Payment address validation and rules

mod token_2022_extensions_test;
mod token_2022_test;

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
