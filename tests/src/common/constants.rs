use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const LOOKUP_TABLES_FILE_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/fixtures/lookup_tables.json");

// ============================================================================
// Network URLs
// ============================================================================

/// Default local Solana RPC URL
pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";

/// Default Kora test server URL
pub const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

// ============================================================================
// Test Keypair Paths
// ============================================================================

/// Fee payer keypair path (local testing only)
pub const FEE_PAYER_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/fee-payer-local.json");

/// Sender keypair path (local testing only)
pub const SENDER_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/sender-local.json");

/// USDC mint keypair path (local testing only)
pub const USDC_MINT_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/usdc-mint-local.json");

/// USDC mint 2022 keypair path (local testing only)
pub const USDC_MINT_2022_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/usdc-mint-2022-local.json");

/// Interest bearing mint keypair path (local testing only)
pub const INTEREST_BEARING_MINT_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/mint-2022-interest-bearing.json");

/// Second signer keypair path (for multi-signer tests)
pub const SIGNER2_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/signer2-local.json");

/// Payment address keypair path (for payment address tests)
pub const PAYMENT_KEYPAIR_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/payment-local.json");

// ============================================================================
// Test Public Keys
// ============================================================================

/// Default recipient public key for tests
pub const RECIPIENT_PUBKEY: &str = "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM";

/// Test disallowed address for lookup table tests
pub const TEST_DISALLOWED_ADDRESS: &str = "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek";

/// Test payment address for paymaster tests
pub const TEST_PAYMENT_ADDRESS: &str = "CWvWnVwqAb9HzqwCGkn4purGEUuu27aNsPQM252uLerV";

/// PYUSD token mint on devnet
pub const PYUSD_MINT: &str = "CXk2AMBfi3TwaEL2468s6zP8xq9NxTXjp9gjMgzeUynM";

// ============================================================================
// Test Configuration
// ============================================================================

/// Test USDC mint decimals
pub const TEST_USDC_MINT_DECIMALS: u8 = 6;

// ============================================================================
// Authentication Test Constants
// ============================================================================

/// Test API key for authentication tests
pub const TEST_API_KEY: &str = "test-api-key-123";

/// Test HMAC secret for authentication tests
pub const TEST_HMAC_SECRET: &str = "test-hmac-secret-456";

// ============================================================================
// Helper Functions
// ============================================================================

/// Get recipient pubkey as Pubkey type
pub fn get_recipient_pubkey() -> Pubkey {
    Pubkey::from_str(RECIPIENT_PUBKEY).expect("Invalid recipient pubkey")
}

/// Get test disallowed address as Pubkey type
pub fn get_test_disallowed_pubkey() -> Pubkey {
    Pubkey::from_str(TEST_DISALLOWED_ADDRESS).expect("Invalid disallowed address")
}

/// Get test payment address as Pubkey type
pub fn get_test_payment_pubkey() -> Pubkey {
    Pubkey::from_str(TEST_PAYMENT_ADDRESS).expect("Invalid payment address")
}

/// Get PYUSD mint as Pubkey type
pub fn get_pyusd_mint_pubkey() -> Pubkey {
    Pubkey::from_str(PYUSD_MINT).expect("Invalid PYUSD mint")
}

/// Default fee for a transaction with 2 signers (5000 lamports each)
/// This is used for a lot of tests that only has sender and fee payer as signers
pub fn get_fee_for_default_transaction_in_usdc() -> u64 {
    // 10 000 USDC priced at default 0.001 SOL / USDC (Mock pricing) (6 decimals), so 0.01 USDC
    // 10 000 lamports required (2 x 5000 for signatures) (9 decimals), so 0.00001 SOL
    //
    // Required SOL amount is 0.01 (usdc amount) * 0.001 (usdc price) = 0.00001 SOL
    // Required lamports is 0.00001 SOL * 10^9 (lamports per SOL) = 10 000 lamports
    10_000
}
