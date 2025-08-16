// Integration tests for Kora RPC server (without auth)
//
// CONFIG: Uses tests/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: Core RPC functionality, token operations, compute budget, oracle integrations
//        - RPC methods (estimateTransactionFee, signTransaction, etc.)
//        - Token2022 and SPL token handling
//        - Transaction validation and fee calculation
//        - Oracle price feeds and lookup table resolution

mod compute_budget_integration_tests;
mod rpc_integration_tests;
mod token_integration_tests;

// Include integrations tests
#[path = "../integrations/mod.rs"]
mod integrations;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
