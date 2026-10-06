// create_account_only_via Tests
//
// CONFIG: Uses tests/src/common/fixtures/create-account-only-via-test.toml
//         (free pricing, allow_create_account = true, create_account_only_via = [CPI forwarder])
// TESTS: Fee payer funds account creation only inside the forwarder's CPIs

mod fee_payer_funded_creation;

#[path = "../src/common/mod.rs"]
mod common;

use common::{harness_context, KoraSpec, TestContext};

pub async fn ctx() -> TestContext {
    harness_context(KoraSpec {
        config: "tests/src/common/fixtures/create-account-only-via-test.toml",
        signers: "tests/src/common/fixtures/signers.toml",
        initialize_payments_atas: false,
    })
    .await
}
