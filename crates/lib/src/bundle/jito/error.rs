use thiserror::Error;

/// Jito-specific errors
#[derive(Debug, Error)]
pub enum JitoError {
    #[error("Jito API error: {0}")]
    ApiError(String),

    #[error("Jito bundles not enabled in configuration")]
    NotEnabled,

    #[error("Bundle exceeds maximum size of {0} transactions")]
    BundleTooLarge(usize),

    #[error("Bundle payment insufficient: required {0} lamports, found {1} lamports")]
    InsufficientBundlePayment(u64, u64),
}
