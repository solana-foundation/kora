use super::{solana_signer::SolanaMemorySigner, vault_signer::VaultSigner};
use crate::{
    error::KoraError,
    signer::{privy::types::PrivySigner, turnkey::types::TurnkeySigner},
};
use solana_sdk::{
    pubkey::Pubkey, signature::Signature as SolanaSignature, transaction::VersionedTransaction,
};
use std::error::Error;

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

    fn sign(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl std::future::Future<Output = Result<Signature, Self::Error>> + Send;

    fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl std::future::Future<Output = Result<SolanaSignature, Self::Error>> + Send;
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum KoraSigner {
    Memory(SolanaMemorySigner),
    Turnkey(TurnkeySigner),
    Vault(VaultSigner),
    Privy(PrivySigner),
}

impl KoraSigner {
    pub fn solana_pubkey(&self) -> Pubkey {
        match self {
            KoraSigner::Memory(signer) => signer.solana_pubkey(),
            KoraSigner::Turnkey(signer) => signer.solana_pubkey(),
            KoraSigner::Vault(signer) => signer.solana_pubkey(),
            KoraSigner::Privy(signer) => signer.solana_pubkey(),
        }
    }
}

impl super::Signer for KoraSigner {
    type Error = KoraError;

    async fn sign(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<super::Signature, Self::Error> {
        match self {
            // Some signers expect the serialized message, others expect the message bytes
            KoraSigner::Memory(signer) => signer.sign(transaction).await,
            KoraSigner::Turnkey(signer) => {
                let sig = signer.sign(transaction).await?;
                Ok(super::Signature { bytes: sig, is_partial: false })
            }
            KoraSigner::Vault(signer) => signer.sign(transaction).await,
            KoraSigner::Privy(signer) => {
                let sig = signer.sign(transaction).await?;
                Ok(super::Signature { bytes: sig, is_partial: false })
            }
        }
    }

    async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<SolanaSignature, Self::Error> {
        match self {
            // Some signers expect the serialized message, others expect the message bytes
            KoraSigner::Memory(signer) => signer.sign_solana(transaction).await,
            KoraSigner::Vault(signer) => signer.sign_solana(transaction).await,
            KoraSigner::Turnkey(signer) => {
                signer.sign_solana(transaction).await.map_err(KoraError::from)
            }
            KoraSigner::Privy(signer) => {
                signer.sign_solana(transaction).await.map_err(KoraError::from)
            }
        }
    }
}
