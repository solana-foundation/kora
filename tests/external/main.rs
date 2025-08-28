// External Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: External system integrations and dependencies
//        - Oracle price feed integration
//        - Address lookup table resolution
//        - External API interactions

mod jupiter_integration;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
