// ============================================================================
// Network URLs
// ============================================================================

/// Default local Solana RPC URL
pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";

/// Default Kora test server URL
pub const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

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

/// Transfer hook program ID
pub const TRANSFER_HOOK_PROGRAM_ID: &str = "Bcdikjss8HWzKEuj6gEQoFq9TCnGnk6v3kUnRU1gb6hA";

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
// Test Environment Variables
// ============================================================================

/// Test server URL environment variable
pub const TEST_SERVER_URL_ENV: &str = "TEST_SERVER_URL";

/// RPC URL environment variable
pub const RPC_URL_ENV: &str = "RPC_URL";

/// KORA private key environment variable
pub const KORA_PRIVATE_KEY_ENV: &str = "KORA_PRIVATE_KEY";

/// Signer 2 private key environment variable
pub const SIGNER_2_KEYPAIR_ENV: &str = "SIGNER_2_KEYPAIR";

/// Test sender private key environment variable
pub const TEST_SENDER_KEYPAIR_ENV: &str = "TEST_SENDER_KEYPAIR";

/// Test recipient public key environment variable
pub const TEST_RECIPIENT_PUBKEY_ENV: &str = "TEST_RECIPIENT_PUBKEY";

/// Test USDC mint private key environment variable
pub const TEST_USDC_MINT_KEYPAIR_ENV: &str = "TEST_USDC_MINT_KEYPAIR";

/// Test USDC mint decimals environment variable
pub const TEST_USDC_MINT_DECIMALS_ENV: &str = "TEST_USDC_MINT_DECIMALS";

/// Test USDC mint 2022 private key environment variable
pub const TEST_USDC_MINT_2022_KEYPAIR_ENV: &str = "TEST_USDC_MINT_2022_KEYPAIR";

/// Test interest bearing mint private key environment variable
pub const TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV: &str = "TEST_INTEREST_BEARING_MINT_KEYPAIR";

/// Test transfer hook mint private key environment variable
pub const TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV: &str = "TEST_TRANSFER_HOOK_MINT_KEYPAIR";

/// Payment address keypair environment variable
pub const PAYMENT_ADDRESS_KEYPAIR_ENV: &str = "PAYMENT_ADDRESS_KEYPAIR";

/// Test allowed lookup table address environment variable
pub const TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV: &str = "TEST_ALLOWED_LOOKUP_TABLE_ADDRESS";

/// Test disallowed lookup table address environment variable
pub const TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV: &str = "TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS";

/// Test transaction lookup table address environment variable
pub const TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV: &str = "TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS";
