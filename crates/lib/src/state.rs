use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::{atomic::AtomicPtr, Arc};
use std::sync::atomic::Ordering;

use crate::{config::Config, error::KoraError, signer::KoraSigner};

static GLOBAL_SIGNER: Lazy<Arc<RwLock<Option<KoraSigner>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

// Global config with zero-cost reads and hot-reload capability
static GLOBAL_CONFIG: AtomicPtr<Config> = AtomicPtr::new(std::ptr::null_mut());

/// Initialize the global signer with a KoraSigner instance
pub fn init_signer(signer: KoraSigner) -> Result<(), KoraError> {
    let mut signer_guard = GLOBAL_SIGNER.write();
    if signer_guard.is_some() {
        return Err(KoraError::InternalServerError("Signer already initialized".to_string()));
    }

    *signer_guard = Some(signer);
    Ok(())
}

/// Get a reference to the global signer
pub fn get_signer() -> Result<Arc<KoraSigner>, KoraError> {
    let signer_guard = GLOBAL_SIGNER.read();
    match &*signer_guard {
        Some(signer) => Ok(Arc::new(signer.clone())),
        None => Err(KoraError::InternalServerError("Signer not initialized".to_string())),
    }
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

/// Update the global config (dev/test only)
#[cfg(feature = "tests")]
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
