use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;
use utoipa::ToSchema;

use crate::{
    constant::DEFAULT_MAX_TIMESTAMP_AGE, error::KoraError, fee::price::PriceConfig,
    oracle::PriceSource,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
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
    #[serde(default)]
    pub price: PriceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeePayerPolicy {
    pub allow_sol_transfers: bool,
    pub allow_spl_transfers: bool,
    pub allow_token2022_transfers: bool,
    pub allow_assign: bool,
    pub allow_burn: bool,
    pub allow_close_account: bool,
    pub allow_approve: bool,
}

impl Default for FeePayerPolicy {
    fn default() -> Self {
        Self {
            allow_sol_transfers: true,
            allow_spl_transfers: true,
            allow_token2022_transfers: true,
            allow_assign: true,
            allow_burn: true,
            allow_close_account: true,
            allow_approve: true,
        }
    }
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

impl EnabledMethods {
    pub fn iter(&self) -> impl Iterator<Item = bool> {
        [
            self.liveness,
            self.estimate_transaction_fee,
            self.get_supported_tokens,
            self.sign_transaction,
            self.sign_and_send_transaction,
            self.transfer_transaction,
            self.get_blockhash,
            self.get_config,
            self.sign_transaction_if_paid,
        ]
        .into_iter()
    }
}

impl IntoIterator for &EnabledMethods {
    type Item = bool;
    type IntoIter = std::array::IntoIter<bool, 9>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.liveness,
            self.estimate_transaction_fee,
            self.get_supported_tokens,
            self.sign_transaction,
            self.sign_and_send_transaction,
            self.transfer_transaction,
            self.get_blockhash,
            self.get_config,
            self.sign_transaction_if_paid,
        ]
        .into_iter()
    }
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

fn default_max_timestamp_age() -> i64 {
    DEFAULT_MAX_TIMESTAMP_AGE
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KoraConfig {
    pub rate_limit: u64,
    #[serde(default)]
    pub enabled_methods: EnabledMethods,
    pub api_key: Option<String>,
    pub hmac_secret: Option<String>,
    #[serde(default = "default_max_timestamp_age")]
    pub max_timestamp_age: i64,
    // pub redis_url: String,
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
        let contents = fs::read_to_string(path).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to read config file: {e}"))
        })?;

        toml::from_str(&contents).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to parse config file: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::fee::price::PriceModel;

    use super::*;
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

        let config = Config::load_config(temp_file.path()).unwrap();

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

        let config = Config::load_config(temp_file.path()).unwrap();

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

        let result = Config::load_config(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Config::load_config("nonexistent_file.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_margin_price_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1"]
            allowed_tokens = ["token1"]
            allowed_spl_paid_tokens = ["token2"]
            disallowed_accounts = []
            price_source = "Jupiter"

            [validation.price]
            type = "margin"
            margin = 0.1

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = Config::load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            PriceModel::Margin { margin } => {
                assert_eq!(*margin, 0.1);
            }
            _ => panic!("Expected Margin price model"),
        }
    }

    #[test]
    fn test_parse_fixed_price_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1"]
            allowed_tokens = ["token1"]
            allowed_spl_paid_tokens = ["token2"]
            disallowed_accounts = []
            price_source = "Jupiter"

            [validation.price]
            type = "fixed"
            amount = 1000000
            token = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = Config::load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            PriceModel::Fixed { amount, token } => {
                assert_eq!(*amount, 1000000); // Amount as token units, not lamports
                assert_eq!(token, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
            }
            _ => panic!("Expected Fixed price model"),
        }
    }

    #[test]
    fn test_parse_free_price_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1"]
            allowed_tokens = ["token1"]
            allowed_spl_paid_tokens = ["token2"]
            disallowed_accounts = []
            price_source = "Jupiter"

            [validation.price]
            type = "free"

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = Config::load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            PriceModel::Free => {
                // Test passed - Free model has no additional fields
            }
            _ => panic!("Expected Free price model"),
        }
    }

    #[test]
    fn test_parse_missing_price_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1"]
            allowed_tokens = ["token1"]
            allowed_spl_paid_tokens = ["token2"]
            disallowed_accounts = []
            price_source = "Jupiter"

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = Config::load_config(temp_file.path()).unwrap();

        // Should default to Margin with 0.0 margin
        match &config.validation.price.model {
            PriceModel::Margin { margin } => {
                assert_eq!(*margin, 0.0);
            }
            _ => panic!("Expected default Margin price model with 0.0 margin"),
        }
    }

    #[test]
    fn test_parse_invalid_price_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1"]
            allowed_tokens = ["token1"]
            allowed_spl_paid_tokens = ["token2"]
            disallowed_accounts = []
            price_source = "Jupiter"

            [validation.price]
            type = "invalid_type"
            margin = 0.1

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let result = Config::load_config(temp_file.path());
        assert!(result.is_err());

        // Verify it's a parsing error
        if let Err(KoraError::InternalServerError(msg)) = result {
            assert!(msg.contains("Failed to parse config file"));
        } else {
            panic!("Expected InternalServerError with parsing failure message");
        }
    }
}
