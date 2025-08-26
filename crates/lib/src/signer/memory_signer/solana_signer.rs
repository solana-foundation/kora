use crate::error::KoraError;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature as SolanaSignature},
    signer::Signer as SolanaSigner,
    transaction::VersionedTransaction,
};

use crate::signer::{KeypairUtil, Signature};

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
    pub fn solana_pubkey(&self) -> Pubkey {
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
        transaction: &VersionedTransaction,
    ) -> Result<SolanaSignature, KoraError> {
        let solana_sig = self.keypair.sign_message(&transaction.message.serialize());

        let sig_bytes: [u8; 64] = solana_sig
            .as_ref()
            .try_into()
            .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

        Ok(SolanaSignature::from(sig_bytes))
    }

    pub async fn sign(&self, transaction: &VersionedTransaction) -> Result<Signature, KoraError> {
        let solana_sig = self.keypair.sign_message(&transaction.message.serialize());
        Ok(Signature { bytes: solana_sig.as_ref().to_vec(), is_partial: false })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{config_mock::ConfigMockBuilder, transaction_mock::create_mock_transaction};
    use solana_sdk::signer::Signer as SolanaSigner;

    #[test]
    fn test_new_signer() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let expected_pubkey = keypair.pubkey();
        let signer = SolanaMemorySigner::new(keypair);

        assert_eq!(signer.solana_pubkey(), expected_pubkey);
    }

    #[test]
    fn test_from_bytes_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let bytes = keypair.to_bytes();
        let signer = SolanaMemorySigner::from_bytes(&bytes).unwrap();

        assert_eq!(signer.solana_pubkey(), keypair.pubkey());
    }

    #[test]
    fn test_from_bytes_invalid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let invalid_bytes = vec![0u8; 31]; // Wrong length
        let result = SolanaMemorySigner::from_bytes(&invalid_bytes);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::SigningError(_)));
    }

    #[test]
    fn test_pubkey_methods() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let keypair = Keypair::new();
        let expected_pubkey = keypair.pubkey();
        let signer = SolanaMemorySigner::new(keypair);

        assert_eq!(signer.pubkey(), expected_pubkey.to_bytes());
        assert_eq!(signer.solana_pubkey(), expected_pubkey);
        assert_eq!(signer.pubkey_base58(), expected_pubkey.to_string());
    }

    #[test]
    fn test_from_private_key_string_base58() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let test_private_key = "5MaiiCavjCmn9Hs1o3eznqDEhRwxo7pXiAYez7keQUviUkauRiTMD8DrESdrNjN8zd9mTmVhRvBJeg5vhyvgrAhG";
        let signer = SolanaMemorySigner::from_private_key_string(test_private_key).unwrap();

        assert_eq!(signer.pubkey_base58(), "5pVyoAeURQHNMVU7DmfMHvCDNmTEYXWfEwc136GYhTKG");
    }

    #[test]
    fn test_from_private_key_string_invalid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = SolanaMemorySigner::from_private_key_string("invalid_key");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::SigningError(_)));
    }

    #[tokio::test]
    async fn test_sign_solana() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let signer = SolanaMemorySigner::new(Keypair::new());
        let transaction = create_mock_transaction();

        let signature = signer.sign_solana(&transaction).await.unwrap();

        // Verify signature is 64 bytes
        assert_eq!(signature.as_ref().len(), 64);
    }

    #[tokio::test]
    async fn test_sign() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let signer = SolanaMemorySigner::new(Keypair::new());
        let transaction = create_mock_transaction();

        let signature = signer.sign(&transaction).await.unwrap();

        // Verify our custom signature format
        assert_eq!(signature.bytes.len(), 64);
        assert!(!signature.is_partial);
    }

    #[tokio::test]
    async fn test_sign_produces_different_signatures_for_different_transactions() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let signer = SolanaMemorySigner::new(Keypair::new());
        let tx1 = create_mock_transaction();
        let tx2 = create_mock_transaction();

        let sig1 = signer.sign(&tx1).await.unwrap();
        let sig2 = signer.sign(&tx2).await.unwrap();

        // Different transactions should produce different signatures
        assert_ne!(sig1.bytes, sig2.bytes);
    }

    #[tokio::test]
    async fn test_sign_produces_same_signature_for_same_transaction() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let signer = SolanaMemorySigner::new(Keypair::new());
        let transaction = create_mock_transaction();

        let sig1 = signer.sign(&transaction).await.unwrap();
        let sig2 = signer.sign(&transaction).await.unwrap();

        // Same transaction should produce same signature
        assert_eq!(sig1.bytes, sig2.bytes);
    }
}
