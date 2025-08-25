// Multi-Signer Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (multi-signer configuration)
// TESTS: Multi-signer functionality and management
//        - Round-robin signer selection behavior
//        - Signer key consistency across RPC calls
//        - Error handling for invalid/nonexistent signers

mod signer_management;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
