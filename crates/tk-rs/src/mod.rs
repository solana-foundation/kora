use std::str::FromStr;

use base64::Engine;
use p256::ecdsa::signature::Signer;
use reqwest::Client;

mod types;
mod utils;

pub use types::*;
pub use utils::*;

impl TurnkeySigner {
    pub fn new(
        api_public_key: String,
        api_private_key: String,
        organization_id: String,
        private_key_id: String,
        public_key: String,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            api_public_key,
            api_private_key,
            organization_id,
            private_key_id,
            public_key,
            client: Client::new(),
        })
    }

    pub async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let hex_message = hex::encode(message);

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

        let body = serde_json::to_string(&request).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let stamp = self.create_stamp(&body)?;

        let response = self
            .client
            .post("https://api.turnkey.com/public/v1/submit/sign_raw_payload")
            .header("Content-Type", "application/json")
            .header("X-Stamp", stamp)
            .body(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let response = serde_json::from_str::<ActivityResponse>(&response.text().await.unwrap())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if let Some(result) = response.activity.result {
            if let Some(sign_result) = result.sign_raw_payload_result {
                // Decode r and s components
                let r_bytes = hex::decode(&sign_result.r)
                    .map_err(|e| anyhow::anyhow!(format!("Invalid r component: {}", e)))?;
                let s_bytes = hex::decode(&sign_result.s)
                    .map_err(|e| anyhow::anyhow!(format!("Invalid s component: {}", e)))?;

                // Ensure each component is exactly 32 bytes
                if r_bytes.len() > 32 || s_bytes.len() > 32 {
                    return Err(anyhow::anyhow!("Signature component too long"));
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

        Err(anyhow::anyhow!("Failed to get signature from response"))
    }

    pub async fn sign_solana(
        &self,
        message: &[u8],
    ) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
        let sig = self.sign(message).await?;
        let sig_bytes: [u8; 64] = sig.try_into().unwrap();
        Ok(solana_sdk::signature::Signature::from(sig_bytes))
    }

    fn create_stamp(&self, message: &str) -> Result<String, anyhow::Error> {
        let private_key_bytes =
            hex_to_bytes(&self.api_private_key).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let private_key_array: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
        let signing_key = p256::ecdsa::SigningKey::from_slice(&private_key_array)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let signature: p256::ecdsa::Signature = signing_key.sign(message.as_bytes());
        let signature_der = signature.to_der().to_bytes();
        let signature_hex = bytes_to_hex(&signature_der).unwrap();

        let stamp = serde_json::json!({
            "public_key": self.api_public_key,
            "signature": signature_hex,
            "scheme": "SIGNATURE_SCHEME_TK_API_P256"
        });

        let json_stamp =
            serde_json::to_string(&stamp).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json_stamp.as_bytes()))
    }

    pub fn solana_pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        solana_sdk::pubkey::Pubkey::from_str(&self.public_key).unwrap()
    }
}
