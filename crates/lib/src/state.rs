use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};

use crate::{
    config::Config, error::KoraError, signer::SignerPool, transaction::signing_retry_window,
};
use std::time::Duration;

// Global signer pool (for multi-signer support)
static GLOBAL_SIGNER_POOL: Lazy<RwLock<Option<Arc<SignerPool>>>> = Lazy::new(|| RwLock::new(None));

// Global config with zero-cost reads and hot-reload capability
static GLOBAL_CONFIG: AtomicPtr<Config> = AtomicPtr::new(std::ptr::null_mut());

fn sync_probe_lease_from_config(pool: &SignerPool, config: &Config) {
    let sign_timeout = Duration::from_secs(config.kora.sign_timeout_seconds);
    pool.set_probe_lease(signing_retry_window(sign_timeout, config.kora.sign_max_retries));
}

/// Select a request-scoped signer without mutating recovery probe state.
pub fn select_request_signer_with_signer_key(
    signer_key: Option<&str>,
) -> Result<Arc<solana_keychain::Signer>, KoraError> {
    let config = get_config()?;
    let pool = get_signer_pool()?;
    sync_probe_lease_from_config(&pool, config);

    if let Some(signer_key) = signer_key {
        return pool.select_signer_by_pubkey(signer_key);
    }

    pool.select_next_signer()
        .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer from pool: {e}")))
}

/// Reserve a request-scoped signer, acquiring a recovery probe lock if needed.
pub fn get_request_signer_with_signer_key(
    signer_key: Option<&str>,
) -> Result<Arc<solana_keychain::Signer>, KoraError> {
    let config = get_config()?;
    let pool = get_signer_pool()?;
    sync_probe_lease_from_config(&pool, config);

    // If client provided a signer signer_key, try to use that specific signer
    if let Some(signer_key) = signer_key {
        return pool.get_signer_by_pubkey(signer_key);
    }

    // Use configured selection strategy (defaults to round-robin if not specified)
    pool.get_next_signer()
        .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer from pool: {e}")))
}

pub fn reserve_request_signer_by_pubkey(
    signer_pubkey: &solana_sdk::pubkey::Pubkey,
) -> Result<Arc<solana_keychain::Signer>, KoraError> {
    get_request_signer_with_signer_key(Some(&signer_pubkey.to_string()))
}

/// Initialize the global signer pool with a SignerPool instance
pub fn init_signer_pool(pool: SignerPool) -> Result<(), KoraError> {
    let mut pool_guard = GLOBAL_SIGNER_POOL.write();
    if pool_guard.is_some() {
        return Err(KoraError::InternalServerError("Signer pool already initialized".to_string()));
    }

    log::info!(
        "Initializing global signer pool with {} signers using {:?} strategy",
        pool.len(),
        pool.strategy()
    );

    *pool_guard = Some(Arc::new(pool));
    Ok(())
}

/// Get a reference to the global signer pool
pub fn get_signer_pool() -> Result<Arc<SignerPool>, KoraError> {
    let pool_guard = GLOBAL_SIGNER_POOL.read();
    match &*pool_guard {
        Some(pool) => Ok(Arc::clone(pool)),
        None => Err(KoraError::InternalServerError("Signer pool not initialized".to_string())),
    }
}

/// Get information about all signers (for monitoring/debugging)
pub fn get_signers_info() -> Result<Vec<crate::signer::SignerInfo>, KoraError> {
    let pool = get_signer_pool()?;
    Ok(pool.get_signers_info())
}

/// Update the global signer configs with a new config (test only)
#[cfg(test)]
pub fn update_signer_pool(new_pool: SignerPool) -> Result<(), KoraError> {
    let mut pool_guard = GLOBAL_SIGNER_POOL.write();

    *pool_guard = Some(Arc::new(new_pool));

    Ok(())
}

/// Initialize the global config with a Config instance
pub fn init_config(config: Config) -> Result<(), KoraError> {
    let current_ptr = GLOBAL_CONFIG.load(Ordering::Acquire);
    if !current_ptr.is_null() {
        return Err(KoraError::InternalServerError("Config already initialized".to_string()));
    }

    let config_ptr = Box::into_raw(Box::new(config));
    GLOBAL_CONFIG.store(config_ptr, Ordering::Release);
    Ok(())
}

/// Get a reference to the global config (zero-cost read)
pub fn get_config() -> Result<&'static Config, KoraError> {
    let config_ptr = GLOBAL_CONFIG.load(Ordering::Acquire);
    if config_ptr.is_null() {
        return Err(KoraError::InternalServerError("Config not initialized".to_string()));
    }

    // SAFETY: We ensure the pointer is valid and the config lives for the duration of the program
    Ok(unsafe { &*config_ptr })
}

/// Update the global config with a new full config (test only)
#[cfg(test)]
pub fn update_config(new_config: Config) -> Result<(), KoraError> {
    let old_ptr = GLOBAL_CONFIG.load(Ordering::Acquire);
    let new_ptr = Box::into_raw(Box::new(new_config));

    GLOBAL_CONFIG.store(new_ptr, Ordering::Release);

    // Clean up old config if it exists
    if !old_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(old_ptr);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{signer::pool::SignerWithMetadata, tests::config_mock::ConfigMockBuilder};
    use serial_test::serial;
    use solana_keychain::Signer;
    use solana_sdk::signature::Keypair;

    // Touches GLOBAL_CONFIG + GLOBAL_SIGNER_POOL — must serialize against any other test
    // that mutates either via update_config / update_signer_pool, otherwise parallel tests
    // race and the assertion intermittently observes a probe_lease set by another test.
    #[test]
    #[serial]
    fn test_select_request_signer_updates_probe_lease_from_config() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.sign_timeout_seconds = 15;
        config.kora.sign_max_retries = 5;
        update_config(config).unwrap();

        let keypair = Keypair::new();
        let signer = Signer::from_memory(&keypair.to_base58_string()).unwrap();
        let pool = SignerPool::new(vec![SignerWithMetadata::new(
            "test_signer".to_string(),
            Arc::new(signer),
            1,
        )]);
        update_signer_pool(pool).unwrap();

        let _ = select_request_signer_with_signer_key(None).unwrap();

        let pool = get_signer_pool().unwrap();
        assert_eq!(pool.probe_lease(), signing_retry_window(Duration::from_secs(15), 5));
    }
}
