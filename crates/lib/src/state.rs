use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    Arc,
};

use crate::{
    config::Config,
    error::KoraError,
    signer::{KoraSigner, SignerPool},
};

// Global signer pool (for multi-signer support)
static GLOBAL_SIGNER_POOL: Lazy<RwLock<Option<Arc<SignerPool>>>> = Lazy::new(|| RwLock::new(None));

// Global config with zero-cost reads and hot-reload capability
static GLOBAL_CONFIG: AtomicPtr<Config> = AtomicPtr::new(std::ptr::null_mut());

/// Get a request-scoped signer that should be used consistently throughout a single RPC request
/// This advances the round-robin counter once per request for consistent signer usage
pub fn get_request_signer() -> Result<Arc<KoraSigner>, KoraError> {
    get_request_signer_with_hint(None)
}

/// Get a request-scoped signer with optional hint for consistency across related calls
pub fn get_request_signer_with_hint(
    signer_hint: Option<&str>,
) -> Result<Arc<KoraSigner>, KoraError> {
    let pool = get_signer_pool()?;

    // If client provided a signer hint, try to use that specific signer
    if let Some(hint) = signer_hint {
        let signer_meta = pool.get_signer_by_pubkey(hint)?;
        return Ok(Arc::new(signer_meta.signer.clone()));
    }

    // Default behavior: use next signer from round-robin
    let signer_meta = pool.get_next_signer().map_err(|e| {
        KoraError::InternalServerError(format!("Failed to get signer from pool: {e}"))
    })?;
    Ok(Arc::new(signer_meta.signer.clone()))
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

/// Get the next signer from the pool with error handling and metrics tracking
pub fn get_next_signer_with_tracking() -> Result<(Arc<KoraSigner>, String), KoraError> {
    let pool = get_signer_pool()?;
    let signer_meta = pool.get_next_signer()?;
    let signer_name = signer_meta.name.clone();
    Ok((Arc::new(signer_meta.signer.clone()), signer_name))
}

/// Mark a signer operation as successful (for metrics tracking)
pub fn mark_signer_success(signer_name: &str) {
    if let Ok(pool) = get_signer_pool() {
        pool.mark_signer_success(signer_name);
    }
}

/// Mark a signer operation as failed (for metrics tracking)
pub fn mark_signer_error(signer_name: &str) {
    if let Ok(pool) = get_signer_pool() {
        pool.mark_signer_error(signer_name);
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
