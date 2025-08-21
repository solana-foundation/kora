use crate::{
    get_request_signer_with_signer_key,
    signer::{KoraSigner, SignerPool, SignerWithMetadata, SolanaMemorySigner},
    state::{get_config, update_config, update_signer_pool},
    tests::{account_mock, config_mock::ConfigMockBuilder, rpc_mock},
    Config,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

// Re-export mock utilities for easy access
pub use account_mock::*;
pub use rpc_mock::*;

pub fn setup_or_get_test_signer() -> Pubkey {
    if let Ok(signer) = get_request_signer_with_signer_key(None) {
        return signer.solana_pubkey();
    }

    let test_keypair = Keypair::new();

    let signer = SolanaMemorySigner::new(test_keypair.insecure_clone());

    let pool = SignerPool::new(vec![SignerWithMetadata::new(
        "test_signer".to_string(),
        KoraSigner::Memory(signer.clone()),
        1,
    )]);

    match update_signer_pool(pool) {
        Ok(_) => {}
        Err(e) => {
            panic!("Failed to update signer pool: {e}");
        }
    }

    signer.solana_pubkey()
}

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
