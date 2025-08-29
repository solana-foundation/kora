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
