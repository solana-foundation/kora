use std::sync::Arc;
use vaultrs::{client::{VaultClient, VaultClientSettingsBuilder}, transit};
use solana_sdk::{signature::Signature, pubkey::Pubkey};
use bs58;

use crate::common::{error::KoraError, Signer, Signature as KoraSignature};

#[derive(Clone)]
pub struct VaultSigner {
    client: Arc<VaultClient>,
    key_name: String,
    pubkey: Pubkey,
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
                .map_err(|e| KoraError::SigningError(format!("Failed to create Vault client: {}", e)))?
        );

        let pubkey = Pubkey::try_from(bs58::decode(pubkey).into_vec()
            .map_err(|e| KoraError::SigningError(format!("Invalid public key: {}", e)))?.as_slice())
            .map_err(|e| KoraError::SigningError(format!("Invalid public key: {}", e)))?;

        Ok(Self {
            client: Arc::new(client.map_err(|e| KoraError::SigningError(format!("Failed to create Vault client: {}", e)))?),
            key_name,
            pubkey,
        })
    }

    pub fn solana_pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl super::Signer for VaultSigner {
    type Error = KoraError;
    
    async fn sign(&self, message: &[u8]) -> Result<KoraSignature, Self::Error> {
        let signature = transit::data::sign(
            self.client.as_ref(),
            "transit",
            &self.key_name,
            &base64::encode(message),
            None,
        )
        .await
        .map_err(|e| KoraError::SigningError(format!("Failed to sign with Vault: {}", e)))?;

        let sig_bytes = base64::decode(signature.signature)
            .map_err(|e| KoraError::SigningError(format!("Failed to decode signature: {}", e)))?;

        Ok(KoraSignature {
            bytes: sig_bytes,
            is_partial: false,
        })
    }

    async fn sign_solana(&self, message: &[u8]) -> Result<Signature, Self::Error> {
        let signature = transit::data::sign(
            self.client.as_ref(),
            "transit",
            &self.key_name,
            &base64::encode(message),
            None,
        )
        .await
        .map_err(|e| KoraError::SigningError(format!("Failed to sign with Vault: {}", e)))?;

        let sig_bytes = base64::decode(signature.signature)
            .map_err(|e| KoraError::SigningError(format!("Failed to decode signature: {}", e)))?;

        Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| KoraError::SigningError(format!("Invalid signature format: {}", e)))
    }
}