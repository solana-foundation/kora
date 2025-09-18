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
    fn get_privy_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.app_id, self.app_secret);
        format!("Basic {}", STANDARD.encode(credentials))
    }

    /// Get the public key for this wallet
    pub async fn get_public_key(&self) -> Result<Pubkey, PrivyError> {
        let url = format!("{}/wallets/{}", self.api_base_url, self.wallet_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.get_privy_auth_header())
            .header("privy-app-id", &self.app_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            log::error!(
                "Privy API get_public_key error - status: {status}, response: {error_text}"
            );
            return Err(PrivyError::ApiError(status));
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
            .header("Authorization", self.get_privy_auth_header())
            .header("privy-app-id", &self.app_id)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            log::error!("Privy API sign_solana error - status: {status}, response: {error_text}");
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

#[cfg(test)]
mod tests {
    use crate::tests::transaction_mock::create_mock_transaction;

    use super::*;
    use mockito::Server;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_new_privy_signer() {
        let app_id = "test_app_id".to_string();
        let app_secret = "test_app_secret".to_string();
        let wallet_id = "test_wallet_id".to_string();

        let signer = PrivySigner::new(app_id.clone(), app_secret.clone(), wallet_id.clone());

        assert_eq!(signer.app_id, app_id);
        assert_eq!(signer.app_secret, app_secret);
        assert_eq!(signer.wallet_id, wallet_id);
        assert_eq!(signer.api_base_url, "https://api.privy.io/v1");
        assert_eq!(signer.public_key, Pubkey::default());
    }

    #[test]
    fn test_solana_pubkey() {
        let mut signer = PrivySigner::new(
            "app_id".to_string(),
            "app_secret".to_string(),
            "wallet_id".to_string(),
        );

        let test_pubkey = Pubkey::new_unique();
        signer.public_key = test_pubkey;

        assert_eq!(signer.solana_pubkey(), test_pubkey);
    }

    #[test]
    fn test_get_privy_auth_header() {
        let signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "wallet123".to_string(),
        );

        let auth_header = signer.get_privy_auth_header();
        let expected_credentials = "test_app:test_secret";
        let expected_encoded = STANDARD.encode(expected_credentials);
        let expected_header = format!("Basic {expected_encoded}");

        assert_eq!(auth_header, expected_header);
    }

    #[tokio::test]
    async fn test_get_public_key_success() {
        // Setup mock server
        let mut server = Server::new_async().await;

        // Mocked response from Privy API based on https://docs.privy.io/api-reference/wallets/get
        let mock_response = r#"{
            "id": "clz4ndjp705bh14za2p80kt3f",
            "object": "wallet",
            "created_at": 1721937199,
            "address": "11111111111111111111111111111111",
            "chain_type": "solana",
            "chain_id": "solana:101",
            "wallet_client": "privy",
            "wallet_client_type": "privy",
            "connector_type": "embedded",
            "recovery_method": "privy",
            "imported": false,
            "delegated": false,
            "user_id": "did:privy:cm0xlrcmj01ja13m6ncg4ewce"
        }"#;

        // Create mock endpoint for GET /wallets/{wallet_id}
        let _mock = server
            .mock("GET", "/wallets/test_wallet")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create_async()
            .await;

        let mut signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "test_wallet".to_string(),
        );
        signer.api_base_url = server.url();

        // Test successful public key retrieval
        let result = signer.get_public_key().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "11111111111111111111111111111111");
    }

    #[tokio::test]
    async fn test_get_public_key_api_error() {
        let mut server = Server::new_async().await;

        // Mocked error response from Privy API based on https://docs.privy.io/api-reference/wallets/get
        let mock_error_response = r#"{
            "error": {
                "error": "wallet_not_found",
                "error_description": "Wallet not found."
            }
        }"#;

        // Create mock endpoint for GET /wallets/{wallet_id} returning 404 error
        let _mock = server
            .mock("GET", "/wallets/invalid_wallet")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(mock_error_response)
            .create_async()
            .await;

        let mut signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "invalid_wallet".to_string(),
        );
        signer.api_base_url = server.url();

        // Test API error handling
        let result = signer.get_public_key().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivyError::ApiError(404)));
    }

    #[tokio::test]
    async fn test_sign_solana_success() {
        // Setup mock server
        let mut server = Server::new_async().await;

        // Mocked response from Privy RPC API based on https://docs.privy.io/api-reference/wallets/solana/sign-transaction
        // Modified to match the SignTransactionResponse struct
        let mock_response = r#"{
            "method": "signTransaction",
            "data": {
                "signed_transaction": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDArczbMia1tLmq7zz4DinMNN0pJ1JtLdqIJPUw3YrGCzuAXUE8535pRk2d+dzOdFlBIpWfgXa9F2zWLidMUr5zdDlBG2q1y4YJlUDl7ov7FLfWvDlhVAidT5nXu6bJgZG1qNgJQBd55PwKBNYMFYBJ2rIbgNhfHu6E/OmZFpV9EUCuE8AAAABd2J0AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFB8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAA=",
                "encoding": "base64"
            }
        }"#;

        // Create mock endpoint for POST /wallets/{wallet_id}/rpc
        let _mock = server
            .mock("POST", "/wallets/test_wallet/rpc")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "test_wallet".to_string(),
        );
        signer.api_base_url = server.url();

        // Test successful signing
        let result = signer.sign_solana(&test_transaction).await;
        assert!(result.is_ok());
        let signature = result.unwrap();
        assert_eq!(signature.to_string().len(), 64); // Hex encoded signature length
    }

    #[tokio::test]
    async fn test_sign_solana_api_error() {
        let mut server = Server::new_async().await;

        // Mocked error response from Privy RPC API based on https://docs.privy.io/api-reference/wallets/solana/sign-transaction
        let mock_error_response = r#"{
            "error": {
                "error": "invalid_request",
                "error_description": "The transaction is invalid or malformed."
            }
        }"#;

        // Create mock endpoint for POST /wallets/{wallet_id}/rpc returning 400 error
        let _mock = server
            .mock("POST", "/wallets/test_wallet/rpc")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(mock_error_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "test_wallet".to_string(),
        );
        signer.api_base_url = server.url();

        // Test API error handling
        let result = signer.sign_solana(&test_transaction).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PrivyError::ApiError(400)));
    }

    #[tokio::test]
    async fn test_sign_success() {
        // Setup mock server
        let mut server = Server::new_async().await;

        // Mocked response from Privy RPC API (same as sign_solana) based on https://docs.privy.io/api-reference/wallets/solana/sign-transaction
        // Modified to match the SignTransactionResponse struct
        let mock_response = r#"{
            "method": "signTransaction",
            "data": {
                "signed_transaction": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDArczbMia1tLmq7zz4DinMNN0pJ1JtLdqIJPUw3YrGCzuAXUE8535pRk2d+dzOdFlBIpWfgXa9F2zWLidMUr5zdDlBG2q1y4YJlUDl7ov7FLfWvDlhVAidT5nXu6bJgZG1qNgJQBd55PwKBNYMFYBJ2rIbgNhfHu6E/OmZFpV9EUCuE8AAAABd2J0AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFB8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAA=",
                "encoding": "base64"
            }
        }"#;

        // Create mock endpoint for POST /wallets/{wallet_id}/rpc
        let _mock = server
            .mock("POST", "/wallets/test_wallet/rpc")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = PrivySigner::new(
            "test_app".to_string(),
            "test_secret".to_string(),
            "test_wallet".to_string(),
        );
        signer.api_base_url = server.url();

        // Test successful signing returns Vec<u8>
        let result = signer.sign(&test_transaction).await;
        assert!(result.is_ok());
        let signature_bytes = result.unwrap();
        assert_eq!(signature_bytes.len(), 64); // Solana signature is 64 bytes
    }
}
