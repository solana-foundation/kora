// Lighthouse Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/lighthouse-test.toml (lighthouse enabled)
// TESTS: Verifies lighthouse fee payer protection assertions are added to transactions
//        - Single transaction signing with lighthouse assertion (legacy and V0)
//        - Bundle signing with lighthouse assertion (only on last tx)
//
// NOTE: signAndSendTransaction and signAndSendBundle are NOT tested here because
// when lighthouse modifies a transaction, existing signatures become invalid.
// For these flows, clients should use signTransaction/signBundle and sign after
// receiving the modified transaction back from Kora.

mod lighthouse_tests;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
