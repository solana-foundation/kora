use crate::{
    error::KoraError,
    signer::{
        memory_signer::solana_signer::SolanaMemorySigner, privy::types::PrivySigner,
        turnkey::types::TurnkeySigner, vault::vault_signer::VaultSigner,
    },
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

#[derive(Clone, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::{config_mock::ConfigMockBuilder, transaction_mock::create_mock_transaction},
        Signer,
    };
    use solana_sdk::{signature::Keypair, signer::Signer as SolanaSigner};

    #[test]
    fn test_kora_signer_memory_pubkey() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let expected_pubkey = keypair.pubkey();
        let memory_signer = SolanaMemorySigner::new(keypair);
        let kora_signer = KoraSigner::Memory(memory_signer);

        assert_eq!(kora_signer.solana_pubkey(), expected_pubkey);
    }

    #[tokio::test]
    async fn test_kora_signer_memory_sign() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let memory_signer = SolanaMemorySigner::new(keypair);
        let kora_signer = KoraSigner::Memory(memory_signer);
        let transaction = create_mock_transaction();

        let result = kora_signer.sign(&transaction).await;
        assert!(result.is_ok());

        let signature = result.unwrap();
        assert_eq!(signature.bytes.len(), 64);
        assert!(!signature.is_partial);
    }

    #[tokio::test]
    async fn test_kora_signer_memory_sign_solana() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let memory_signer = SolanaMemorySigner::new(keypair);
        let kora_signer = KoraSigner::Memory(memory_signer);
        let transaction = create_mock_transaction();

        let result = kora_signer.sign_solana(&transaction).await;
        assert!(result.is_ok());

        let signature = result.unwrap();
        assert_eq!(signature.as_ref().len(), 64);
    }
}
