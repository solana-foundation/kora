// RPC Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// TESTS: Core JSON-RPC functionality with all transaction variants
//        - Basic endpoints (getConfig, getBlockhash, etc.)
//        - Fee estimation with legacy, V0, V0+lookup, compute budget scenarios
//        - Transaction signing with all formats and conditions
//        - Transfer operations with various token types
//        - Durable transaction blocking (nonce-based transactions)

mod basic_endpoints;
mod compute_budget;
mod durable_transactions;
mod fee_estimation;
mod transaction_signing;
mod transfers;
mod webhook_events;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
