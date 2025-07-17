use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PrivySigner {
    pub app_id: String,
    pub app_secret: String,
    pub wallet_id: String,
    pub api_base_url: String,
    pub client: Client,
    pub public_key: solana_sdk::pubkey::Pubkey,
}

#[derive(Default)]
pub struct PrivyConfig {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub wallet_id: Option<String>,
}

// API request/response types for Privy
#[derive(Serialize)]
pub struct SignTransactionRequest {
    pub method: &'static str,
    pub params: SignTransactionParams,
}

#[derive(Serialize)]
pub struct SignTransactionParams {
    pub transaction: String,
    pub encoding: &'static str,
}

#[derive(Deserialize, Debug)]
pub struct SignTransactionResponse {
    pub method: String,
    pub data: SignTransactionData,
}

#[derive(Deserialize, Debug)]
pub struct SignTransactionData {
    #[serde(rename = "signed_transaction")]
    pub signed_transaction: String,
    pub encoding: String,
}

// Wallet info response
#[derive(Deserialize, Debug)]
pub struct WalletResponse {
    pub id: String,
    pub address: String,
    pub chain_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_client_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hd_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_signers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

// Error types using anyhow
#[derive(Debug)]
pub enum PrivyError {
    MissingConfig(&'static str),
    ApiError(u16),
    InvalidResponse,
    InvalidPublicKey,
    InvalidSignature,
    SerializationError,
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    Base64Error(base64::DecodeError),
    InitializationError,
    RuntimeError,
    Other(anyhow::Error),
}

impl std::fmt::Display for PrivyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivyError::MissingConfig(field) => write!(f, "Missing config: {field}"),
            PrivyError::ApiError(status) => write!(f, "API error: {status}"),
            PrivyError::InvalidResponse => write!(f, "Invalid response"),
            PrivyError::InvalidPublicKey => write!(f, "Invalid public key"),
            PrivyError::InvalidSignature => write!(f, "Invalid signature"),
            PrivyError::SerializationError => write!(f, "Serialization error"),
            PrivyError::RequestError(e) => write!(f, "Request error: {e}"),
            PrivyError::JsonError(e) => write!(f, "JSON error: {e}"),
            PrivyError::Base64Error(e) => write!(f, "Base64 error: {e}"),
            PrivyError::InitializationError => write!(f, "Initialization error"),
            PrivyError::RuntimeError => write!(f, "Runtime error"),
            PrivyError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PrivyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PrivyError::RequestError(e) => Some(e),
            PrivyError::JsonError(e) => Some(e),
            PrivyError::Base64Error(e) => Some(e),
            PrivyError::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for PrivyError {
    fn from(err: reqwest::Error) -> Self {
        PrivyError::RequestError(err)
    }
}

impl From<serde_json::Error> for PrivyError {
    fn from(err: serde_json::Error) -> Self {
        PrivyError::JsonError(err)
    }
}

impl From<base64::DecodeError> for PrivyError {
    fn from(err: base64::DecodeError) -> Self {
        PrivyError::Base64Error(err)
    }
}

impl From<anyhow::Error> for PrivyError {
    fn from(err: anyhow::Error) -> Self {
        PrivyError::Other(err)
    }
}
