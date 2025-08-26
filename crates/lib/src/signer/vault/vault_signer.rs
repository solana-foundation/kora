use base64::{engine::general_purpose::STANDARD, Engine as _};
use bs58;
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use std::sync::Arc;
use vaultrs::{
    client::{VaultClient, VaultClientSettingsBuilder},
    transit,
};

use crate::{error::KoraError, Signature as KoraSignature};

#[derive(Clone)]
pub struct VaultSigner {
    client: Arc<VaultClient>,
    key_name: String,
    pubkey: Pubkey,
}

impl std::fmt::Debug for VaultSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultSigner")
            .field("key_name", &self.key_name)
            .field("pubkey", &self.pubkey)
            .finish()
    }
}

impl VaultSigner {
    pub fn new(
        vault_addr: String,
        token: String,
        key_name: String,
        pubkey: String,
    ) -> Result<Self, KoraError> {
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(vault_addr)
                .token(token)
                .build()
                .map_err(|e| {
                    KoraError::SigningError(format!("Failed to create Vault client: {e}"))
                })?,
        );

        let pubkey = Pubkey::try_from(
            bs58::decode(pubkey)
                .into_vec()
                .map_err(|e| KoraError::SigningError(format!("Invalid public key: {e}")))?
                .as_slice(),
        )
        .map_err(|e| KoraError::SigningError(format!("Invalid public key: {e}")))?;

        Ok(Self {
            client: Arc::new(client.map_err(|e| {
                KoraError::SigningError(format!("Failed to create Vault client: {e}"))
            })?),
            key_name,
            pubkey,
        })
    }

    pub fn solana_pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl VaultSigner {
    pub async fn sign(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<KoraSignature, KoraError> {
        let signature = transit::data::sign(
            self.client.as_ref(),
            "transit",
            &self.key_name,
            &STANDARD.encode(transaction.message.serialize()),
            None,
        )
        .await
        .map_err(|e| KoraError::SigningError(format!("Failed to sign with Vault: {e}")))?;

        let sig_bytes = STANDARD
            .decode(signature.signature)
            .map_err(|e| KoraError::SigningError(format!("Failed to decode signature: {e}")))?;

        Ok(KoraSignature { bytes: sig_bytes, is_partial: false })
    }

    pub async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Signature, KoraError> {
        let signature = transit::data::sign(
            self.client.as_ref(),
            "transit",
            &self.key_name,
            &STANDARD.encode(transaction.message.serialize()),
            None,
        )
        .await
        .map_err(|e| KoraError::SigningError(format!("Failed to sign with Vault: {e}")))?;

        let sig_bytes = STANDARD
            .decode(signature.signature)
            .map_err(|e| KoraError::SigningError(format!("Failed to decode signature: {e}")))?;

        Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| KoraError::SigningError(format!("Invalid signature format: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vault_signer() {
        let vault_addr = "https://vault.example.com".to_string();
        let token = "test_token".to_string();
        let key_name = "test_key".to_string();
        let pubkey = "11111111111111111111111111111111".to_string();

        let result = VaultSigner::new(vault_addr, token, key_name.clone(), pubkey);

        assert!(result.is_ok());
        let signer = result.unwrap();
        assert_eq!(signer.key_name, key_name);
        assert_eq!(signer.pubkey.to_string(), "11111111111111111111111111111111");
    }

    #[test]
    fn test_new_vault_signer_invalid_pubkey() {
        let vault_addr = "https://vault.example.com".to_string();
        let token = "test_token".to_string();
        let key_name = "test_key".to_string();
        let invalid_pubkey = "invalid_pubkey".to_string();

        let result = VaultSigner::new(vault_addr, token, key_name, invalid_pubkey);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid public key"));
    }

    #[test]
    fn test_solana_pubkey() {
        let vault_addr = "https://vault.example.com".to_string();
        let token = "test_token".to_string();
        let key_name = "test_key".to_string();
        let pubkey = "11111111111111111111111111111111".to_string();

        let signer = VaultSigner::new(vault_addr, token, key_name, pubkey).unwrap();
        let retrieved_pubkey = signer.solana_pubkey();

        assert_eq!(retrieved_pubkey.to_string(), "11111111111111111111111111111111");
    }
}
