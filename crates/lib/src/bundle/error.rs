use crate::bundle::jito::JitoError;
use thiserror::Error;

/// Bundle errors
#[derive(Debug, Error)]
pub enum BundleError {
    #[error("Bundle is empty")]
    Empty,

    #[error("Bundle validation failed: {0}")]
    ValidationError(String),

    #[error("Failed to serialize transaction: {0}")]
    SerializationError(String),

    #[error(transparent)]
    Jito(#[from] JitoError),
}
