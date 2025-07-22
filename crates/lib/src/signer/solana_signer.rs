use crate::error::KoraError;
use solana_sdk::{
    signature::{Keypair, Signature as SolanaSignature},
    signer::Signer as SolanaSigner,
    transaction::Transaction,
};

use super::{KeypairUtil, Signature};

/// A Solana-based signer that uses an in-memory keypair
#[derive(Debug)]
pub struct SolanaMemorySigner {
    keypair: Keypair,
}

impl SolanaMemorySigner {
    /// Creates a new signer from a Solana keypair
    pub fn new(keypair: Keypair) -> Self {
        Self { keypair }
    }

    /// Creates a new signer from a private key byte array
    pub fn from_bytes(private_key: &[u8]) -> Result<Self, KoraError> {
        let keypair = Keypair::try_from(private_key)
            .map_err(|e| KoraError::SigningError(format!("Invalid private key bytes: {e}")))?;
        Ok(Self { keypair })
    }

    /// Get the public key of this signer
    pub fn pubkey(&self) -> [u8; 32] {
        self.keypair.pubkey().to_bytes()
    }

    /// Get solana pubkey
    pub fn solana_pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        self.keypair.pubkey()
    }

    /// Get the base58-encoded public key
    pub fn pubkey_base58(&self) -> String {
        bs58::encode(self.pubkey()).into_string()
    }

    /// Creates a new signer from a private key string that can be in multiple formats:
    /// - Base58 encoded string (current format)
    /// - U8Array format: "[0, 1, 2, ...]"
    /// - File path to a JSON keypair file
    pub fn from_private_key_string(private_key: &str) -> Result<Self, KoraError> {
        let keypair = KeypairUtil::from_private_key_string(private_key)?;
        Ok(Self::new(keypair))
    }
}

impl Clone for SolanaMemorySigner {
    fn clone(&self) -> Self {
        Self::from_bytes(&self.keypair.to_bytes()).expect("Failed to clone keypair")
    }
}

impl SolanaMemorySigner {
    pub async fn sign_solana(
        &self,
        transaction: &Transaction,
    ) -> Result<SolanaSignature, KoraError> {
        let solana_sig = self.keypair.sign_message(&transaction.message_data());

        let sig_bytes: [u8; 64] = solana_sig
            .as_ref()
            .try_into()
            .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

        Ok(SolanaSignature::from(sig_bytes))
    }

    pub async fn sign(&self, transaction: &Transaction) -> Result<Signature, KoraError> {
        let solana_sig = self.keypair.sign_message(&transaction.message_data());
        Ok(Signature { bytes: solana_sig.as_ref().to_vec(), is_partial: false })
    }
}
