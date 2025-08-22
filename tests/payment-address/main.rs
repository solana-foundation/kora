// Payment address tests for Kora RPC server
//
// CONFIG: Uses tests/common/fixtures/paymaster-address-test.toml
// TESTS: Special payment address functionality and ATA initialization
//        - Payment address-based transaction signing
//        - Associated Token Account (ATA) creation and initialization
//        - Token transfer validation with specific payment addresses
//        - Fee payer policy enforcement for payment scenarios

mod payment_address_tests;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
