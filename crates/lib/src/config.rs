use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;
use utoipa::ToSchema;

use solana_client::nonblocking::rpc_client::RpcClient;

use crate::{error::KoraError, oracle::PriceSource, token::check_valid_tokens};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeePayerPolicy {
    pub allow_sol_transfers: bool,
    pub allow_spl_transfers: bool,
    pub allow_token2022_transfers: bool,
    pub allow_assign: bool,
}

impl Default for FeePayerPolicy {
    fn default() -> Self {
        Self {
            allow_sol_transfers: true,
            allow_spl_transfers: true,
            allow_token2022_transfers: true,
            allow_assign: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationConfig {
    pub max_allowed_lamports: u64,
    pub max_signatures: u64,
    pub allowed_programs: Vec<String>,
    pub allowed_tokens: Vec<String>,
    pub allowed_spl_paid_tokens: Vec<String>,
    pub disallowed_accounts: Vec<String>,
    pub price_source: PriceSource,
    #[serde(default)] // Default for backward compatibility
    pub fee_payer_policy: FeePayerPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnabledMethods {
    pub liveness: bool,
    pub estimate_transaction_fee: bool,
    pub get_supported_tokens: bool,
    pub sign_transaction: bool,
    pub sign_and_send_transaction: bool,
    pub transfer_transaction: bool,
    pub get_blockhash: bool,
    pub get_config: bool,
    pub sign_transaction_if_paid: bool,
}

impl Default for EnabledMethods {
    fn default() -> Self {
        Self {
            liveness: true,
            estimate_transaction_fee: true,
            get_supported_tokens: true,
            sign_transaction: true,
            sign_and_send_transaction: true,
            transfer_transaction: true,
            get_blockhash: true,
            get_config: true,
            sign_transaction_if_paid: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoraConfig {
    pub rate_limit: u64,
    #[serde(default)]
    pub enabled_methods: EnabledMethods,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
    let contents = fs::read_to_string(path)
        .map_err(|e| KoraError::InternalServerError(format!("Failed to read config file: {e}")))?;

    toml::from_str(&contents)
        .map_err(|e| KoraError::InternalServerError(format!("Failed to parse config file: {e}")))
}

impl Config {
    pub async fn validate(&self, _rpc_client: &RpcClient) -> Result<(), KoraError> {
        if self.validation.allowed_tokens.is_empty() {
            return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
        }

        check_valid_tokens(&self.validation.allowed_tokens)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::oracle::PriceSource;

    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1", "program2"]
            allowed_tokens = ["token1", "token2"]
            allowed_spl_paid_tokens = ["token3"]
            disallowed_accounts = ["account1"]
            price_source = "Jupiter"
            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = load_config(temp_file.path()).unwrap();

        assert_eq!(config.validation.max_allowed_lamports, 1000000000);
        assert_eq!(config.validation.max_signatures, 10);
        assert_eq!(config.validation.allowed_programs, vec!["program1", "program2"]);
        assert_eq!(config.validation.allowed_tokens, vec!["token1", "token2"]);
        assert_eq!(config.validation.allowed_spl_paid_tokens, vec!["token3"]);
        assert_eq!(config.validation.disallowed_accounts, vec!["account1"]);
        assert_eq!(config.validation.price_source, PriceSource::Jupiter);
        assert_eq!(config.kora.rate_limit, 100);
        // Test default enabled methods
        assert!(config.kora.enabled_methods.estimate_transaction_fee);
        assert!(config.kora.enabled_methods.sign_and_send_transaction);
    }

    #[test]
    fn test_load_config_with_enabled_methods() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1", "program2"]
            allowed_tokens = ["token1", "token2"]
            allowed_spl_paid_tokens = ["token3"]
            disallowed_accounts = ["account1"]
            price_source = "Jupiter"
            [kora]
            rate_limit = 100
            [kora.enabled_methods]
            liveness = true
            estimate_transaction_fee = false
            get_supported_tokens = true
            sign_transaction = true
            sign_and_send_transaction = false
            transfer_transaction = true
            get_blockhash = true
            get_config = true
            sign_transaction_if_paid = true
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = load_config(temp_file.path()).unwrap();

        assert_eq!(config.kora.rate_limit, 100);
        assert!(config.kora.enabled_methods.liveness);
        assert!(!config.kora.enabled_methods.estimate_transaction_fee);
        assert!(config.kora.enabled_methods.get_supported_tokens);
        assert!(config.kora.enabled_methods.sign_transaction);
        assert!(!config.kora.enabled_methods.sign_and_send_transaction);
        assert!(config.kora.enabled_methods.transfer_transaction);
        assert!(config.kora.enabled_methods.get_blockhash);
        assert!(config.kora.enabled_methods.get_config);
        assert!(config.kora.enabled_methods.sign_transaction_if_paid);
    }

    #[test]
    fn test_load_invalid_config() {
        let invalid_content = "invalid toml content";
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, invalid_content).unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_config("nonexistent_file.toml");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config() {
        let mut config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1000000000,
                max_signatures: 10,
                allowed_programs: vec!["program1".to_string()],
                allowed_tokens: vec!["token1".to_string()],
                allowed_spl_paid_tokens: vec!["token3".to_string()],
                disallowed_accounts: vec!["account1".to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
            },
            kora: KoraConfig { rate_limit: 100, enabled_methods: EnabledMethods::default() },
        };

        // Test empty tokens list
        config.validation.allowed_tokens.clear();
        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate(&rpc_client).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InternalServerError(_)));
    }
}
