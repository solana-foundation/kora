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
    pub caip2: &'static str,
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
    pub signature: String,
}

// Wallet info response
#[derive(Deserialize, Debug)]
pub struct WalletResponse {
    pub id: String,
    pub address: String,
    pub chain_type: String,
    pub wallet_client_type: String,
    pub connector_type: Option<String>,
    pub imported: bool,
    pub delegated: bool,
    pub hd_path: Option<String>,
    pub public_key: Option<String>,
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
