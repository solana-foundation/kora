use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PrivySigner {
    pub app_id: String,
    pub app_secret: String,
    pub wallet_id: String,
    pub api_base_url: String,
    pub client: Client,
    pub public_key: Option<solana_sdk::pubkey::Pubkey>,
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

// Error types
#[derive(thiserror::Error, Debug)]
pub enum PrivyError {
    #[error("Missing config: {0}")]
    MissingConfig(&'static str),

    #[error("API error: {0}")]
    ApiError(u16),

    #[error("Invalid response")]
    InvalidResponse,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Base64 error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}
