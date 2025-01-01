use std::error::Error;
use once_cell::sync::Lazy;
use solana_sdk::signature::Signature as SolanaSignature;

use super::{error::KoraError, solana_signer::SolanaMemorySigner, tk::TurnkeySigner};

static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")
});

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

#[derive(Clone)]
pub enum KoraSigner {
    Memory(SolanaMemorySigner),
    Turnkey(TurnkeySigner),
}

impl KoraSigner {
    pub fn solana_pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        match self {
            KoraSigner::Memory(signer) => signer.solana_pubkey(),
            KoraSigner::Turnkey(signer) => signer.solana_pubkey(),
        }
    }
}

impl super::Signer for KoraSigner {
    type Error = KoraError;

    fn partial_sign(&self, message: &[u8]) -> Result<super::Signature, Self::Error> {
        match self {
            KoraSigner::Memory(signer) => signer.partial_sign(message),
            KoraSigner::Turnkey(signer) => {
                let bytes = RUNTIME.block_on(signer.partial_sign(message))?;
                Ok(super::Signature { bytes, is_partial: false })
            }
        }
    }

    fn full_sign(&self, message: &[u8]) -> Result<super::Signature, Self::Error> {
        match self {
            KoraSigner::Memory(signer) => signer.full_sign(message),
            KoraSigner::Turnkey(signer) => {
                let bytes = RUNTIME.block_on(signer.full_sign(message))?;
                Ok(super::Signature { bytes, is_partial: false })
            }
        }
    }

    fn partial_sign_solana(&self, message: &[u8]) -> Result<solana_sdk::signature::Signature, Self::Error> {
        match self {
            KoraSigner::Memory(signer) => signer.partial_sign_solana(message),
            KoraSigner::Turnkey(signer) => {
                let bytes = RUNTIME.block_on(signer.partial_sign_solana(message))?;
                let sig_bytes: [u8; 64] = bytes.try_into()
                    .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;
                Ok(solana_sdk::signature::Signature::from(sig_bytes))
            }
        }
    }
}
