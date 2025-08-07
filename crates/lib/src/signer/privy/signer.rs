use base64::{engine::general_purpose::STANDARD, Engine};
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use std::str::FromStr;

use crate::signer::privy::types::{
    PrivyError, PrivySigner, SignTransactionParams, SignTransactionRequest,
    SignTransactionResponse, WalletResponse,
};

impl PrivySigner {
    /// Create a new PrivySigner
    pub fn new(app_id: String, app_secret: String, wallet_id: String) -> Self {
        Self {
            app_id,
            app_secret,
            wallet_id,
            api_base_url: "https://api.privy.io/v1".to_string(),
            client: reqwest::Client::new(),
            public_key: Pubkey::default(),
        }
    }

    /// Initialize the signer by fetching the public key
    pub async fn init(&mut self) -> Result<(), PrivyError> {
        let pubkey = self.get_public_key().await?;
        self.public_key = pubkey;
        Ok(())
    }

    /// Get the cached public key
    pub fn solana_pubkey(&self) -> Pubkey {
        self.public_key
    }

    /// Get the Basic Auth header value
    fn get_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.app_id, self.app_secret);
        format!("Basic {}", STANDARD.encode(credentials))
    }

    /// Get the public key for this wallet
    pub async fn get_public_key(&self) -> Result<Pubkey, PrivyError> {
        let url = format!("{}/wallets/{}", self.api_base_url, self.wallet_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .header("privy-app-id", &self.app_id)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(PrivyError::ApiError(response.status().as_u16()));
        }

        let wallet_info: WalletResponse = response.json().await?;

        // For Solana wallets, the address is the public key
        Pubkey::from_str(&wallet_info.address).map_err(|_| PrivyError::InvalidPublicKey)
    }

    /// Sign a transaction
    ///
    /// The transaction parameter should be a fully serialized Solana transaction
    /// (including empty signature placeholders), not just the message bytes.
    pub async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Signature, PrivyError> {
        let url = format!("{}/wallets/{}/rpc", self.api_base_url, self.wallet_id);
        let serialized =
            bincode::serialize(transaction).map_err(|_| PrivyError::SerializationError)?;
        let request = SignTransactionRequest {
            method: "signTransaction",
            params: SignTransactionParams {
                transaction: STANDARD.encode(serialized),
                encoding: "base64",
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.get_auth_header())
            .header("privy-app-id", &self.app_id)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(PrivyError::ApiError(status));
        }

        let response_text = response.text().await?;

        let sign_response: SignTransactionResponse = serde_json::from_str(&response_text)?;

        // Decode the signed transaction from base64
        let signed_tx_bytes = STANDARD.decode(&sign_response.data.signed_transaction)?;

        // Deserialize the transaction to extract the signature
        let signed_tx: VersionedTransaction =
            bincode::deserialize(&signed_tx_bytes).map_err(|_| PrivyError::InvalidResponse)?;

        // Get the first signature (which should be the one we just created)
        if let Some(signature) = signed_tx.signatures.first() {
            Ok(*signature)
        } else {
            Err(PrivyError::InvalidSignature)
        }
    }

    pub async fn sign(&self, transaction: &VersionedTransaction) -> Result<Vec<u8>, PrivyError> {
        let signature = self.sign_solana(transaction).await?;
        Ok(signature.as_ref().to_vec())
    }
}
