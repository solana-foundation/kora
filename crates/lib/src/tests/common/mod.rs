use std::sync::Arc;

/// Common test utilities and centralized re-exports
///
/// This module provides:
/// 1. Setup functions for test environment initialization (signer & config)
/// 2. Centralized re-exports of commonly used mock utilities
use crate::{
    get_request_signer_with_signer_key,
    signer::{pool::SignerWithMetadata, SignerPool},
    state::{get_config, update_config, update_signer_pool},
    tests::{account_mock, config_mock::ConfigMockBuilder, rpc_mock},
    usage_limit::UsageTracker,
    Config, KoraError,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

// Re-export mock utilities for centralized access
pub use account_mock::*;
pub use rpc_mock::*;
use solana_signers::{Signer, SolanaSigner};

/// Setup or retrieve test signer for global state initialization
///
/// Returns the signer's public key.
pub fn setup_or_get_test_signer() -> Pubkey {
    if let Ok(signer) = get_request_signer_with_signer_key(None) {
        return signer.pubkey();
    }

    let test_keypair = Keypair::new();

    // Create external signer and wrap with adapter
    let external_signer = Signer::from_memory(&test_keypair.to_base58_string()).unwrap();

    let pool = SignerPool::new(vec![SignerWithMetadata::new(
        "test_signer".to_string(),
        Arc::new(external_signer),
        1,
    )]);

    match update_signer_pool(pool) {
        Ok(_) => {}
        Err(e) => {
            panic!("Failed to update signer pool: {e}");
        }
    }

    solana_sdk::signer::Signer::pubkey(&test_keypair)
}

/// Setup or retrieve test config for global state initialization
///
/// Returns the config object.
pub fn setup_or_get_test_config() -> Config {
    if let Ok(config) = get_config() {
        return config.clone();
    }

    let config = ConfigMockBuilder::new().build();

    match update_config(config.clone()) {
        Ok(_) => config.clone(),
        Err(e) => {
            panic!("Failed to initialize config: {e}");
        }
    }
}

/// Initialize or update the global usage limiter (test only)
///
/// This function ignores "already initialized" errors for test flexibility.
/// Usage limiter initialization is optional and will not fail tests if unavailable.
pub async fn setup_or_get_test_usage_limiter() -> Result<(), KoraError> {
    match UsageTracker::init_usage_limiter().await {
        Ok(()) => Ok(()),
        Err(KoraError::InternalServerError(ref msg)) if msg.contains("already initialized") => {
            // In tests, ignore the already initialized error
            // The limiter is already set up from a previous test
            Ok(())
        }
        Err(e) => Err(e),
    }
}
