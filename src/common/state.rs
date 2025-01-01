use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::Arc;

use super::{error::KoraError, signer::KoraSigner};

static GLOBAL_SIGNER: Lazy<Arc<RwLock<Option<KoraSigner>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

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
