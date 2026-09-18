mod free_signing_tests;

#[path = "../src/common/mod.rs"]
mod common;

use common::{harness_context, KoraSpec, TestContext};

pub async fn ctx() -> TestContext {
    harness_context(KoraSpec {
        config: "tests/src/common/fixtures/kora-free-test.toml",
        signers: "tests/src/common/fixtures/signers.toml",
        initialize_payments_atas: false,
    })
    .await
}
