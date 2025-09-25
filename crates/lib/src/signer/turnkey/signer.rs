use base64::Engine;
use p256::ecdsa::signature::Signer;
use reqwest::Client;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use solana_sdk::transaction::VersionedTransaction;

use crate::signer::{
    turnkey::types::{ActivityResponse, SignParameters, SignRequest, TurnkeyError, TurnkeySigner},
    utils::{bytes_to_hex, hex_to_bytes},
};

impl TurnkeySigner {
    pub fn new(
        api_public_key: String,
        api_private_key: String,
        organization_id: String,
        private_key_id: String,
        public_key: String,
        pubkey: Pubkey,
    ) -> Self {
        Self {
            api_public_key,
            api_private_key,
            organization_id,
            private_key_id,
            public_key,
            pubkey,
            api_base_url: "https://api.turnkey.com".to_string(),
            client: Client::new(),
        }
    }

    pub async fn sign(&self, transaction: &VersionedTransaction) -> Result<Vec<u8>, TurnkeyError> {
        let hex_message = hex::encode(transaction.message.serialize());

        let request = SignRequest {
            activity_type: "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis().to_string(),
            organization_id: self.organization_id.clone(),
            parameters: SignParameters {
                sign_with: self.private_key_id.clone(),
                payload: hex_message,
                encoding: "PAYLOAD_ENCODING_HEXADECIMAL".to_string(),
                hash_function: "HASH_FUNCTION_NOT_APPLICABLE".to_string(),
            },
        };

        let body = serde_json::to_string(&request).map_err(TurnkeyError::JsonError)?;

        let stamp = self.create_stamp(&body)?;

        let url = format!("{}/public/v1/submit/sign_raw_payload", self.api_base_url);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Stamp", stamp)
            .body(body)
            .send()
            .await
            .map_err(TurnkeyError::RequestError)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            log::error!("Turnkey API error - status: {status}, response: {error_text}");
            return Err(TurnkeyError::ApiError(status));
        }

        let response_text = response.text().await.map_err(TurnkeyError::RequestError)?;

        let response = serde_json::from_str::<ActivityResponse>(&response_text)
            .map_err(TurnkeyError::JsonError)?;

        if let Some(result) = response.activity.result {
            if let Some(sign_result) = result.sign_raw_payload_result {
                // Decode r and s components
                let r_bytes = hex::decode(&sign_result.r).map_err(TurnkeyError::InvalidHex)?;
                let s_bytes = hex::decode(&sign_result.s).map_err(TurnkeyError::InvalidHex)?;

                // Ensure each component is exactly 32 bytes
                if r_bytes.len() > 32 || s_bytes.len() > 32 {
                    return Err(TurnkeyError::InvalidSignature);
                }

                // Create properly padded 32-byte arrays
                let mut final_r = [0u8; 32];
                let mut final_s = [0u8; 32];

                // Copy bytes with proper padding (right-aligned)
                final_r[32 - r_bytes.len()..].copy_from_slice(&r_bytes);
                final_s[32 - s_bytes.len()..].copy_from_slice(&s_bytes);

                // Combine r and s into final 64-byte signature
                let mut signature = Vec::with_capacity(64);
                signature.extend_from_slice(&final_r);
                signature.extend_from_slice(&final_s);

                return Ok(signature);
            }
        }

        Err(TurnkeyError::InvalidResponse)
    }

    pub async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Signature, TurnkeyError> {
        let sig = self.sign(transaction).await?;
        let sig_bytes: [u8; 64] =
            sig.try_into().map_err(|_| TurnkeyError::InvalidSignatureLength)?;
        Ok(Signature::from(sig_bytes))
    }

    fn create_stamp(&self, message: &str) -> Result<String, TurnkeyError> {
        let private_key_bytes =
            hex_to_bytes(&self.api_private_key).map_err(TurnkeyError::InvalidStamp)?;
        let private_key_array: [u8; 32] =
            private_key_bytes.try_into().map_err(|_| TurnkeyError::InvalidPrivateKeyLength)?;
        let signing_key = p256::ecdsa::SigningKey::from_slice(&private_key_array)
            .map_err(TurnkeyError::SigningKeyError)?;

        let signature: p256::ecdsa::Signature = signing_key.sign(message.as_bytes());
        let signature_der = signature.to_der().to_bytes();
        let signature_hex = bytes_to_hex(&signature_der).map_err(TurnkeyError::InvalidStamp)?;

        let stamp = serde_json::json!({
            "public_key": self.api_public_key,
            "signature": signature_hex,
            "scheme": "SIGNATURE_SCHEME_TK_API_P256"
        });

        let json_stamp = serde_json::to_string(&stamp).map_err(TurnkeyError::JsonError)?;

        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json_stamp.as_bytes()))
    }

    pub fn solana_pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::tests::transaction_mock::create_mock_transaction;

    use super::*;
    use mockito::Server;

    #[test]
    fn test_new_turnkey_signer() {
        let api_public_key = "test_api_public_key".to_string();
        let api_private_key = "test_api_private_key".to_string();
        let organization_id = "test_org_id".to_string();
        let private_key_id = "test_private_key_id".to_string();
        let public_key = "11111111111111111111111111111111".to_string();

        let signer = TurnkeySigner::new(
            api_public_key.clone(),
            api_private_key.clone(),
            organization_id.clone(),
            private_key_id.clone(),
            public_key.clone(),
            Pubkey::from_str(&public_key).unwrap(),
        );

        assert_eq!(signer.api_public_key, api_public_key);
        assert_eq!(signer.api_private_key, api_private_key);
        assert_eq!(signer.organization_id, organization_id);
        assert_eq!(signer.private_key_id, private_key_id);
        assert_eq!(signer.public_key, public_key);
    }

    #[test]
    fn test_solana_pubkey_valid() {
        let signer = TurnkeySigner::new(
            "api_pub".to_string(),
            "api_priv".to_string(),
            "org".to_string(),
            "key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );

        let pubkey = signer.solana_pubkey();
        assert_eq!(pubkey.to_string(), "11111111111111111111111111111111");
    }

    #[tokio::test]
    async fn test_sign_success() {
        let mut server = Server::new_async().await;

        // Mocked response from Turnkey API based on https://docs.turnkey.com/api-reference/activities/sign-raw-payload
        let mock_response = r#"{
            "activity": {
                "id": "12345678-1234-1234-1234-123456789012",
                "organizationId": "test_org_id",
                "status": "ACTIVITY_STATUS_COMPLETED",
                "type": "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
                "timestampMs": "1640995200000",
                "result": {
                    "signRawPayloadResult": {
                        "r": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                        "s": "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
                    }
                }
            }
        }"#;

        let _mock = server
            .mock("POST", "/public/v1/submit/sign_raw_payload")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = TurnkeySigner::new(
            "test_api_public_key".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(), // Valid hex private key
            "test_org_id".to_string(),
            "test_private_key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );
        signer.api_base_url = server.url();

        let result = signer.sign(&test_transaction).await;
        assert!(result.is_ok());
        let signature_bytes = result.unwrap();
        assert_eq!(signature_bytes.len(), 64); // Combined r + s components
    }

    #[tokio::test]
    async fn test_sign_api_error() {
        let mut server = Server::new_async().await;

        // Mocked error response from Turnkey API
        // For API errors, we'll return an activity with no result to trigger "Failed to get signature from response"
        let mock_error_response = r#"{
            "activity": {
                "id": "12345678-1234-1234-1234-123456789012",
                "organizationId": "test_org_id",
                "status": "ACTIVITY_STATUS_FAILED",
                "type": "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
                "timestampMs": "1640995200000"
            }
        }"#;

        // Create mock endpoint for POST /public/v1/submit/sign_raw_payload returning 400 error
        let _mock = server
            .mock("POST", "/public/v1/submit/sign_raw_payload")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(mock_error_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = TurnkeySigner::new(
            "invalid_api_public_key".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(), // Valid hex private key format
            "invalid_org_id".to_string(),
            "invalid_private_key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );

        signer.api_base_url = server.url();

        // Test API error handling
        let result = signer.sign(&test_transaction).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TurnkeyError::ApiError(_)));
    }

    #[tokio::test]
    async fn test_sign_rate_limit_error() {
        let mut server = Server::new_async().await;

        let rate_limit_response = r#"{
            "code": 8,
            "message": "",
            "details": [],
            "turnkeyErrorCode": ""
        }"#;

        let _mock = server
            .mock("POST", "/public/v1/submit/sign_raw_payload")
            .with_status(429)
            .with_header("content-type", "application/json")
            .with_body(rate_limit_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = TurnkeySigner::new(
            "test_api_public_key".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            "test_org_id".to_string(),
            "test_private_key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );

        signer.api_base_url = server.url();

        // Test that 429 rate limit is properly handled
        let result = signer.sign(&test_transaction).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            TurnkeyError::ApiError(status) => {
                assert_eq!(status, 429);
            }
            _ => panic!("Expected ApiError with 429 status"),
        }
    }

    #[tokio::test]
    async fn test_sign_solana_success() {
        let mut server = Server::new_async().await;

        // Mocked response from Turnkey API (same as sign)
        let mock_response = r#"{
            "activity": {
                "id": "12345678-1234-1234-1234-123456789012",
                "organizationId": "test_org_id",
                "status": "ACTIVITY_STATUS_COMPLETED",
                "type": "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
                "timestampMs": "1640995200000",
                "result": {
                    "signRawPayloadResult": {
                        "r": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                        "s": "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
                    }
                }
            }
        }"#;

        let _mock = server
            .mock("POST", "/public/v1/submit/sign_raw_payload")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create_async()
            .await;

        let test_transaction = create_mock_transaction();

        let mut signer = TurnkeySigner::new(
            "test_api_public_key".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(), // Valid hex private key
            "test_org_id".to_string(),
            "test_private_key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );

        signer.api_base_url = server.url();

        // Test successful signing returns Signature
        let result = signer.sign_solana(&test_transaction).await;
        assert!(result.is_ok());
        let signature = result.unwrap();
        // Check signature length as string representation
        assert_eq!(signature.to_string().len(), 87); // Base58 encoded signature length
    }

    #[test]
    fn test_create_stamp() {
        let signer = TurnkeySigner::new(
            "test_api_public_key".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(), // Valid hex private key
            "test_org_id".to_string(),
            "test_private_key_id".to_string(),
            "11111111111111111111111111111111".to_string(),
            Pubkey::from_str(&"11111111111111111111111111111111").unwrap(),
        );

        let test_message = r#"{"test": "message"}"#;

        let result = signer.create_stamp(test_message);
        assert!(result.is_ok());
        let stamp = result.unwrap();

        // Stamp should be base64 encoded JSON containing public_key, signature, scheme
        assert!(!stamp.is_empty());

        // Decode and verify stamp structure
        let decoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(stamp.as_bytes()).unwrap();
        let stamp_json: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(stamp_json["public_key"], "test_api_public_key");
        assert_eq!(stamp_json["scheme"], "SIGNATURE_SCHEME_TK_API_P256");
        assert!(stamp_json["signature"].is_string());
    }

    #[test]
    fn test_signature_component_padding() {
        // Test the signature component padding logic
        // Turnkey returns variable-length r and s components that need padding to 32 bytes

        // Test cases for different component lengths:
        // - Short components (< 32 bytes) should be left-padded with zeros
        // - Full 32-byte components should remain unchanged
        // - Components > 32 bytes should be rejected with error

        // Test short components (< 32 bytes) - should be left-padded with zeros
        let short_r = "1234"; // 2 bytes hex = 1 byte actual
        let r_bytes = hex::decode(short_r).unwrap();
        assert!(r_bytes.len() < 32);

        // Create properly padded 32-byte array (simulating the logic from sign method)
        let mut final_r = [0u8; 32];
        final_r[32 - r_bytes.len()..].copy_from_slice(&r_bytes);

        // Verify padding: should have zeros at the beginning
        assert_eq!(final_r[0], 0);
        assert_eq!(final_r[30], 0x12); // First byte of "1234"
        assert_eq!(final_r[31], 0x34); // Second byte of "1234"

        // Test full 32-byte components - should remain unchanged
        let full_r = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"; // 32 bytes
        let full_r_bytes = hex::decode(full_r).unwrap();
        assert_eq!(full_r_bytes.len(), 32);

        let mut final_full_r = [0u8; 32];
        final_full_r[32 - full_r_bytes.len()..].copy_from_slice(&full_r_bytes);
        assert_eq!(&final_full_r[..], &full_r_bytes[..]);

        // Test components > 32 bytes - should be rejected
        let long_r = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"; // 33 bytes
        let long_r_bytes = hex::decode(long_r).unwrap();
        assert!(long_r_bytes.len() > 32);
    }
}
