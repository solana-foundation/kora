use hex::FromHexError;
use p256::ecdsa::signature;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TurnkeySigner {
    pub organization_id: String,
    pub private_key_id: String,
    pub api_public_key: String,
    pub api_private_key: String,
    pub public_key: String,
    pub api_base_url: String,
    pub client: Client,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignRequest {
    #[serde(rename = "type")]
    pub activity_type: String,
    pub timestamp_ms: String,
    pub organization_id: String,
    pub parameters: SignParameters,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignParameters {
    pub sign_with: String,
    pub payload: String,
    pub encoding: String,
    pub hash_function: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    pub activity: Activity,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub result: Option<ActivityResult>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResult {
    pub sign_raw_payload_result: Option<SignResult>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignResult {
    pub r: String,
    pub s: String,
}

#[derive(Debug)]
pub enum TurnkeyError {
    ApiError(u16),
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    InvalidSignature,
    InvalidHex(FromHexError),
    InvalidStamp(anyhow::Error),
    SigningKeyError(signature::Error),
    InvalidResponse,
    InvalidPrivateKeyLength,
    InvalidPublicKey,
    Other(anyhow::Error),
}

impl std::fmt::Display for TurnkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnkeyError::ApiError(status) => write!(f, "API error: {status}"),
            TurnkeyError::InvalidResponse => write!(f, "Invalid response"),
            TurnkeyError::InvalidPublicKey => write!(f, "Invalid public key"),
            TurnkeyError::InvalidSignature => write!(f, "Invalid signature"),
            TurnkeyError::InvalidPrivateKeyLength => write!(f, "Invalid private key length"),
            TurnkeyError::RequestError(e) => write!(f, "Request error: {e}"),
            TurnkeyError::JsonError(e) => write!(f, "JSON error: {e}"),
            TurnkeyError::InvalidStamp(e) => write!(f, "Invalid stamp: {e}"),
            TurnkeyError::InvalidHex(e) => write!(f, "Invalid Hex: {e}"),
            TurnkeyError::SigningKeyError(e) => write!(f, "Signing key error: {e}"),
            TurnkeyError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TurnkeyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turnkey_error_display() {
        let error = TurnkeyError::ApiError(429);
        assert_eq!(error.to_string(), "API error: 429");

        let error = TurnkeyError::InvalidResponse;
        assert_eq!(error.to_string(), "Invalid response");

        let error = TurnkeyError::InvalidSignature;
        assert_eq!(error.to_string(), "Invalid signature");
    }

    #[test]
    fn test_turnkey_error_conversion_to_kora_error() {
        use crate::error::KoraError;

        let turnkey_error = TurnkeyError::ApiError(429);
        let kora_error: KoraError = turnkey_error.into();

        match kora_error {
            KoraError::SigningError(msg) => {
                assert_eq!(msg, "API error: 429");
            }
            _ => panic!("Expected SigningError"),
        }
    }
}
