use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

use crate::common::{token::check_valid_tokens, KoraError};
use solana_client::nonblocking::rpc_client::RpcClient;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub max_allowed_lamports: u64,
    pub max_signatures: usize,
    pub allowed_programs: Vec<String>,
    pub allowed_tokens: Vec<String>,
    pub allowed_spl_paid_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoraConfig {
    pub rate_limit: u64,
    pub redis_url: String,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        KoraError::InternalServerError(format!("Failed to read config file: {}", e))
    })?;

    toml::from_str(&contents)
        .map_err(|e| KoraError::InternalServerError(format!("Failed to parse config file: {}", e)))
}

impl Config {
    pub async fn validate(&self, rpc_client: &RpcClient) -> Result<(), KoraError> {
        if self.validation.allowed_tokens.is_empty() {
            log::error!("No tokens enabled");
            return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
        }

        check_valid_tokens(rpc_client, &self.validation.allowed_tokens).await?;
        Ok(())
    }
}
