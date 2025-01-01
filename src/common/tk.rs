use {
    p256::ecdsa::signature::Signer, reqwest::Client, serde::{Deserialize, Serialize}, std::str::FromStr
};
use super::error::KoraError;

#[derive(Clone)]
pub struct TurnkeySigner {
    organization_id: String,
    private_key_id: String,
    api_public_key: String,
    api_private_key: String,
    client: Client,
}

#[derive(Serialize)]
struct SignRequest {
    activity_type: String,
    timestamp_ms: String,
    organization_id: String,
    parameters: SignParameters,
}

#[derive(Serialize)]
struct SignParameters {
    sign_with: String,
    payload: String,
    encoding: String,
    hash_function: String,
}

#[derive(Deserialize)]
struct ActivityResponse {
    activity: Activity,
}

#[derive(Deserialize)]
struct Activity {
    result: Option<ActivityResult>,
}

#[derive(Deserialize)]
struct ActivityResult {
    sign_raw_payload_result: Option<SignResult>,
}

#[derive(Deserialize)]
struct SignResult {
    r: String,
    s: String,
}

impl TurnkeySigner {
    pub fn new(
        api_public_key: String,
        api_private_key: String,
        organization_id: String,
        private_key_id: String,
    ) -> Result<Self, KoraError> {
        Ok(Self {
            api_public_key,
            api_private_key,
            organization_id,
            private_key_id,
            client: Client::new(),
        })
    }

    pub async fn partial_sign_solana(&self, message: &[u8]) -> Result<Vec<u8>, KoraError> {
        self.partial_sign(message).await
    }

    pub async fn full_sign(&self, message: &[u8]) -> Result<Vec<u8>, KoraError> {
        self.partial_sign(message).await
    }

    pub async fn partial_sign(&self, message: &[u8]) -> Result<Vec<u8>, KoraError> {
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

        let body = serde_json::to_string(&request).map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;
        let stamp = self.create_stamp(&body)?;

        let response = self.client
            .post("https://api.turnkey.com/public/v1/submit/sign_raw_payload")
            .header("Content-Type", "application/json")
            .header("X-Stamp", stamp)
            .body(body)
            .send()
            .await
            .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?
            .json::<ActivityResponse>()
            .await
            .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;

        if let Some(result) = response.activity.result {
            if let Some(sign_result) = result.sign_raw_payload_result {
                let signature = format!("{}{}", sign_result.r, sign_result.s);
                return Ok(hex::decode(signature).map_err(|e| KoraError::InvalidTransaction(e.to_string()))?);
            }
        }

        Err(KoraError::InvalidTransaction("Failed to get signature from response".to_string()))
    }

    fn create_stamp(&self, message: &str) -> Result<String, KoraError> {
        let private_key_bytes = hex::decode(&self.api_private_key).map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;
        let signing_key = p256::ecdsa::SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;
        
        let signature: p256::ecdsa::Signature = signing_key.sign(message.as_bytes());
        let signature_der = signature.to_der().to_bytes();
        let signature_hex = hex::encode(signature_der);

        let stamp = serde_json::json!({
            "public_key": self.api_public_key,
            "signature": signature_hex,
            "scheme": "SIGNATURE_SCHEME_TK_API_P256"
        });

        let json_stamp = serde_json::to_string(&stamp).map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;
        Ok(base64::encode(&json_stamp))
    }

    pub fn solana_pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        solana_sdk::pubkey::Pubkey::from_str(&self.api_public_key).unwrap()
    }
}
