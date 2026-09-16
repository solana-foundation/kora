// Adversarial Basic Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (permissive policies)
// TESTS: Security and robustness testing with normal configuration
//        - Program validation attacks (disallowed programs)
//        - Invalid token states (frozen)
//        - Fee payer exploitation
//        - Request body size limit (DDOS protection)

mod body_size_limit;
mod fee_payer_exploitation;
mod program_validation;
mod token_states;

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
