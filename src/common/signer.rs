use std::error::Error;

use solana_sdk::signature::Signature as SolanaSignature;

/// Represents a signature for a message
#[derive(Debug, Clone)]
pub struct Signature {
    /// The raw bytes of the signature
    pub bytes: Vec<u8>,
    /// Whether this is a partial signature or a complete signature
    pub is_partial: bool,
}

/// A trait for signing arbitrary messages
pub trait Signer {
    /// The error type returned by signing operations
    type Error: Error + Send + Sync + 'static;

    /// Partially signs a message, typically used in multi-signature scenarios
    /// Returns a partial signature that can be combined with other signatures
    fn partial_sign(&self, message: &[u8]) -> Result<Signature, Self::Error>;

    /// Fully signs a message, producing a complete signature
    /// This is used when a single signature is sufficient
    fn full_sign(&self, message: &[u8]) -> Result<Signature, Self::Error>;

    /// Partially signs a message, producing a Solana signature
    fn partial_sign_solana(&self, message: &[u8]) -> Result<SolanaSignature, Self::Error>;
}
