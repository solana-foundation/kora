use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, system_program::ID as SYSTEM_PROGRAM_ID};
use spl_token::ID as SPL_TOKEN_PROGRAM_ID;
use spl_token_2022::ID as TOKEN_2022_PROGRAM_ID;
use std::{fs, path::Path, str::FromStr};
use toml;
use utoipa::ToSchema;

use crate::{
    account::{validate_account, AccountType},
    constant::DEFAULT_MAX_TIMESTAMP_AGE,
    error::KoraError,
    oracle::PriceSource,
    token::check_valid_tokens,
    transaction::{PriceConfig, PriceModel},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub port: u16,
    pub scrape_interval: u64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { enabled: false, endpoint: "/metrics".to_string(), port: 8080, scrape_interval: 60 }
    }
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

#[cfg(test)]
impl ValidationConfig {
    pub fn test_default() -> Self {
        Self {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            price_source: PriceSource::Mock,
            fee_payer_policy: FeePayerPolicy::default(),
            price: PriceConfig::default(),
        }
    }

    pub fn with_price_source(mut self, price_source: PriceSource) -> Self {
        self.price_source = price_source;
        self
    }

    pub fn with_allowed_programs(mut self, programs: Vec<String>) -> Self {
        self.allowed_programs = programs;
        self
    }

    pub fn with_fee_payer_policy(mut self, policy: FeePayerPolicy) -> Self {
        self.fee_payer_policy = policy;
        self
    }

    pub fn with_max_allowed_lamports(mut self, lamports: u64) -> Self {
        self.max_allowed_lamports = lamports;
        self
    }

    pub fn with_max_signatures(mut self, signatures: u64) -> Self {
        self.max_signatures = signatures;
        self
    }

    pub fn with_allowed_tokens(mut self, tokens: Vec<String>) -> Self {
        self.allowed_tokens = tokens;
        self
    }

    pub fn with_allowed_spl_paid_tokens(mut self, tokens: Vec<String>) -> Self {
        self.allowed_spl_paid_tokens = tokens;
        self
    }

    pub fn with_disallowed_accounts(mut self, accounts: Vec<String>) -> Self {
        self.disallowed_accounts = accounts;
        self
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

    pub async fn validate_with_result(
        &self,
        rpc_client: &RpcClient,
        skip_rpc_validation: bool,
    ) -> Result<Vec<String>, Vec<String>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate rate limit (warn if 0)
        if self.kora.rate_limit == 0 {
            warnings.push("Rate limit is set to 0 - this will block all requests".to_string());
        }

        // Validate enabled methods (warn if all false)
        let methods = &self.kora.enabled_methods;
        if !methods.iter().any(|enabled| enabled) {
            warnings.push(
                "All rpc methods are disabled - this will block all functionality".to_string(),
            );
        }

        // Validate max allowed lamports (warn if 0)
        if self.validation.max_allowed_lamports == 0 {
            warnings
                .push("Max allowed lamports is 0 - this will block all SOL transfers".to_string());
        }

        // Validate max signatures (warn if 0)
        if self.validation.max_signatures == 0 {
            warnings.push("Max signatures is 0 - this will block all transactions".to_string());
        }

        // Validate price source (warn if Mock)
        if matches!(self.validation.price_source, PriceSource::Mock) {
            warnings.push("Using Mock price source - not suitable for production".to_string());
        }

        // Validate allowed programs (warn if empty or missing system/token programs)
        if self.validation.allowed_programs.is_empty() {
            warnings.push(
                "No allowed programs configured - this will block all transactions".to_string(),
            );
        } else {
            if !self.validation.allowed_programs.contains(&SYSTEM_PROGRAM_ID.to_string()) {
                warnings.push("Missing System Program in allowed programs - SOL transfers and account operations will be blocked".to_string());
            }
            if !self.validation.allowed_programs.contains(&SPL_TOKEN_PROGRAM_ID.to_string())
                && !self.validation.allowed_programs.contains(&TOKEN_2022_PROGRAM_ID.to_string())
            {
                warnings.push("Missing Token Program in allowed programs - SPL token operations will be blocked".to_string());
            }
        }

        // Validate allowed tokens
        if self.validation.allowed_tokens.is_empty() {
            errors.push("No allowed tokens configured".to_string());
        } else if let Err(e) = check_valid_tokens(&self.validation.allowed_tokens) {
            errors.push(format!("Invalid token address: {e}"));
        }

        // Validate allowed spl paid tokens
        if let Err(e) = check_valid_tokens(&self.validation.allowed_spl_paid_tokens) {
            errors.push(format!("Invalid spl paid token address: {e}"));
        }

        // Validate disallowed accounts
        if let Err(e) = check_valid_tokens(&self.validation.disallowed_accounts) {
            errors.push(format!("Invalid disallowed account address: {e}"));
        }

        // Check if fees are enabled (not Free pricing)
        let fees_enabled = !matches!(self.validation.price.model, PriceModel::Free);

        if fees_enabled {
            // If fees enabled, token or token22 must be enabled in allowed_programs
            let has_token_program =
                self.validation.allowed_programs.contains(&SPL_TOKEN_PROGRAM_ID.to_string());
            let has_token22_program =
                self.validation.allowed_programs.contains(&TOKEN_2022_PROGRAM_ID.to_string());

            if !has_token_program && !has_token22_program {
                errors.push("When fees are enabled, at least one token program (SPL Token or Token2022) must be in allowed_programs".to_string());
            }

            // If fees enabled, allowed_spl_paid_tokens can't be empty
            if self.validation.allowed_spl_paid_tokens.is_empty() {
                errors.push(
                    "When fees are enabled, allowed_spl_paid_tokens cannot be empty".to_string(),
                );
            }
        }

        // Validate that all tokens in allowed_spl_paid_tokens are also in allowed_tokens
        for paid_token in &self.validation.allowed_spl_paid_tokens {
            if !self.validation.allowed_tokens.contains(paid_token) {
                errors.push(format!(
                    "Token {paid_token} in allowed_spl_paid_tokens must also be in allowed_tokens"
                ));
            }
        }

        // Validate margin (error if negative)
        match &self.validation.price.model {
            PriceModel::Fixed { amount, token } => {
                if *amount == 0 {
                    warnings
                        .push("Fixed price amount is 0 - transactions will be free".to_string());
                }
                if Pubkey::from_str(token).is_err() {
                    errors.push(format!("Invalid token address for fixed price: {token}"));
                }
                if !self.validation.allowed_spl_paid_tokens.contains(token) {
                    errors.push(format!(
                        "Token address for fixed price is not in allowed spl paid tokens: {token}"
                    ));
                }
            }
            PriceModel::Margin { margin } => {
                if *margin < 0.0 {
                    errors.push("Margin cannot be negative".to_string());
                } else if *margin > 1.0 {
                    warnings.push(format!("Margin is {}% - this is very high", margin * 100.0));
                }
            }
            _ => {}
        };

        // RPC validation - only if not skipped
        if !skip_rpc_validation {
            // Validate allowed programs - should be executable
            for program_str in &self.validation.allowed_programs {
                if let Ok(program_pubkey) = Pubkey::from_str(program_str) {
                    if let Err(e) =
                        validate_account(rpc_client, &program_pubkey, Some(AccountType::Program))
                            .await
                    {
                        errors.push(format!("Program {program_str} validation failed: {e}"));
                    }
                }
            }

            // Validate allowed tokens - should be non-executable token mints
            for token_str in &self.validation.allowed_tokens {
                if let Ok(token_pubkey) = Pubkey::from_str(token_str) {
                    if let Err(e) =
                        validate_account(rpc_client, &token_pubkey, Some(AccountType::Mint)).await
                    {
                        errors.push(format!("Token {token_str} validation failed: {e}"));
                    }
                }
            }

            // Validate allowed spl paid tokens - should be non-executable token mints
            for token_str in &self.validation.allowed_spl_paid_tokens {
                if let Ok(token_pubkey) = Pubkey::from_str(token_str) {
                    if let Err(e) =
                        validate_account(rpc_client, &token_pubkey, Some(AccountType::Mint)).await
                    {
                        errors.push(format!("SPL paid token {token_str} validation failed: {e}"));
                    }
                }
            }
        }

        // Output results
        println!("=== Configuration Validation ===");
        if errors.is_empty() {
            println!("✓ Configuration validation successful!");
            println!("\n=== Current Configuration ===");
            println!("{self:#?}");
        } else {
            println!("✗ Configuration validation failed!");
            println!("\n❌ Errors:");
            for error in &errors {
                println!("   - {error}");
            }
            println!("\nPlease fix the configuration errors above before deploying.");
        }

        if !warnings.is_empty() {
            println!("\n⚠️  Warnings:");
            for warning in &warnings {
                println!("   - {warning}");
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{constant::DEFAULT_MAX_TIMESTAMP_AGE, oracle::PriceSource};

    use super::*;
    use base64::Engine;
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use solana_program::program_pack::Pack;
    use solana_sdk::account::Account;
    use spl_token::state::Mint;
    use std::{collections::HashMap, fs, sync::Arc};
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
                price: PriceConfig::default(),
            },
            kora: KoraConfig {
                rate_limit: 100,
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
                enabled_methods: EnabledMethods::default(),
            },
            metrics: MetricsConfig::default(),
        };

        // Test empty tokens list
        config.validation.allowed_tokens.clear();
        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate(&rpc_client).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InternalServerError(_)));
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

        let config = load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            crate::transaction::PriceModel::Margin { margin } => {
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

        let config = load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            crate::transaction::PriceModel::Fixed { amount, token } => {
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

        let config = load_config(temp_file.path()).unwrap();

        match &config.validation.price.model {
            crate::transaction::PriceModel::Free => {
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

        let config = load_config(temp_file.path()).unwrap();

        // Should default to Margin with 0.0 margin
        match &config.validation.price.model {
            crate::transaction::PriceModel::Margin { margin } => {
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

        let result = load_config(temp_file.path());
        assert!(result.is_err());

        // Verify it's a parsing error
        if let Err(KoraError::InternalServerError(msg)) = result {
            assert!(msg.contains("Failed to parse config file"));
        } else {
            panic!("Expected InternalServerError with parsing failure message");
        }
    }

    #[tokio::test]
    async fn test_validate_with_result_successful_config() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_validate_with_result_warnings() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 0,  // Should warn
                max_signatures: 0,        // Should warn
                allowed_programs: vec![], // Should warn
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock, // Should warn
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 0, // Should warn
                enabled_methods: EnabledMethods {
                    liveness: false,
                    estimate_transaction_fee: false,
                    get_supported_tokens: false,
                    sign_transaction: false,
                    sign_and_send_transaction: false,
                    transfer_transaction: false,
                    get_blockhash: false,
                    get_config: false,
                    sign_transaction_if_paid: false, // All false - should warn
                },
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Rate limit is set to 0")));
        assert!(warnings.iter().any(|w| w.contains("All rpc methods are disabled")));
        assert!(warnings.iter().any(|w| w.contains("Max allowed lamports is 0")));
        assert!(warnings.iter().any(|w| w.contains("Max signatures is 0")));
        assert!(warnings.iter().any(|w| w.contains("Using Mock price source")));
        assert!(warnings.iter().any(|w| w.contains("No allowed programs configured")));
    }

    #[tokio::test]
    async fn test_validate_with_result_missing_system_program_warning() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec!["SomeOtherProgram".to_string()], // Missing system program
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings.iter().any(|w| w.contains("Missing System Program in allowed programs")));
        assert!(warnings.iter().any(|w| w.contains("Missing Token Program in allowed programs")));
    }

    #[tokio::test]
    async fn test_validate_with_result_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec![], // Error - no tokens
                allowed_spl_paid_tokens: vec!["invalid_token_address".to_string()], // Error - invalid token
                disallowed_accounts: vec!["invalid_account_address".to_string()], // Error - invalid account
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Margin { margin: -0.1 }, // Error - negative margin
                },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("No allowed tokens configured")));
        assert!(errors.iter().any(|e| e.contains("Invalid spl paid token address")));
        assert!(errors.iter().any(|e| e.contains("Invalid disallowed account address")));
        assert!(errors.iter().any(|e| e.contains("Margin cannot be negative")));
    }

    #[tokio::test]
    async fn test_validate_with_result_fixed_price_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0,                                  // Should warn
                        token: "invalid_token_address".to_string(), // Should error
                    },
                },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("Invalid token address for fixed price")));
    }

    #[tokio::test]
    async fn test_validate_with_result_fixed_price_not_in_allowed_tokens() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 1000,
                        token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // Valid but not in allowed
                    },
                },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(
            errors
                .iter()
                .any(|e| e
                    .contains("Token address for fixed price is not in allowed spl paid tokens"))
        );
    }

    #[tokio::test]
    async fn test_validate_with_result_fixed_price_zero_amount_warning() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0, // Should warn
                        token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                    },
                },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings
            .iter()
            .any(|w| w.contains("Fixed price amount is 0 - transactions will be free")));
    }

    #[tokio::test]
    async fn test_validate_with_result_fee_validation_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()], // Missing token programs
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![], // Empty when fees enabled - should error
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("When fees are enabled, at least one token program (SPL Token or Token2022) must be in allowed_programs")));
        assert!(errors
            .iter()
            .any(|e| e.contains("When fees are enabled, allowed_spl_paid_tokens cannot be empty")));
    }

    #[tokio::test]
    async fn test_validate_with_result_paid_tokens_not_in_allowed_tokens() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // Not in allowed_tokens
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("Token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v in allowed_spl_paid_tokens must also be in allowed_tokens")));
    }

    // Helper functions for mocking RPC responses
    fn create_mock_program_account() -> Account {
        Account {
            lamports: 1000000,
            data: vec![0u8; 100],        // Program data
            owner: Pubkey::new_unique(), // Programs are owned by the loader
            executable: true,            // Programs are executable
            rent_epoch: 0,
        }
    }

    fn create_mock_non_executable_account() -> Account {
        Account {
            lamports: 1000000,
            data: vec![0u8; 100],
            owner: Pubkey::new_unique(),
            executable: false, // Not executable
            rent_epoch: 0,
        }
    }

    fn create_mock_token_mint_account() -> Account {
        let mut mint_data = vec![0u8; Mint::LEN];
        let mint = Mint {
            mint_authority: Some(Pubkey::new_unique()).into(),
            supply: 1000000u64.into(),
            decimals: 6,
            is_initialized: true,
            freeze_authority: None.into(),
        };
        Mint::pack(mint, &mut mint_data).unwrap();

        Account {
            lamports: 1000000,
            data: mint_data,
            owner: spl_token::id(), // Token mint owned by SPL Token program
            executable: false,      // Mints are not executable
            rent_epoch: 0,
        }
    }

    fn create_mock_rpc_client_with_account(account: Account) -> Arc<RpcClient> {
        let mut mocks = HashMap::new();
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&account.data);

        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                    "rentEpoch": account.rent_epoch
                }
            }),
        );

        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    fn create_mock_rpc_client_account_not_found() -> Arc<RpcClient> {
        let mut mocks = HashMap::new();
        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": { "slot": 1 },
                "value": null
            }),
        );

        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    // Helper to create a simple test that only validates programs (no tokens)
    fn create_program_only_config() -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()], // Required to pass basic validation
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        }
    }

    // Helper to create a simple test that only validates tokens (no programs)
    fn create_token_only_config() -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![], // No programs
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![], // Empty to avoid duplicate validation
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_validate_with_result_rpc_validation_valid_program() {
        let config = create_program_only_config();

        let mock_account = create_mock_program_account();
        let rpc_client = create_mock_rpc_client_with_account(mock_account);

        // Test with RPC validation enabled (skip_rpc_validation = false)
        // The program validation should pass, but token validation will fail (AccountNotFound)
        let result = config.validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have token validation errors (account not found), but no program validation errors
        assert!(errors.iter().any(|e| e.contains("Token")
            && e.contains("validation failed")
            && e.contains("AccountNotFound")));
        assert!(!errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    async fn test_validate_with_result_rpc_validation_valid_token_mint() {
        let config = create_token_only_config();

        let mock_account = create_mock_token_mint_account();
        let rpc_client = create_mock_rpc_client_with_account(mock_account);

        // Test with RPC validation enabled (skip_rpc_validation = false)
        // Token validation should pass (mock returns token mint) since we have no programs
        let result = config.validate_with_result(&rpc_client, false).await;
        assert!(result.is_ok());
        // Should have warnings about no programs but no errors
        let warnings = result.unwrap();
        assert!(warnings.iter().any(|w| w.contains("No allowed programs configured")));
    }

    #[tokio::test]
    async fn test_validate_with_result_rpc_validation_non_executable_program_fails() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let mock_account = create_mock_non_executable_account(); // Non-executable
        let rpc_client = create_mock_rpc_client_with_account(mock_account);

        // Test with RPC validation enabled (skip_rpc_validation = false)
        let result = config.validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    async fn test_validate_with_result_rpc_validation_account_not_found_fails() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        let rpc_client = create_mock_rpc_client_account_not_found();

        // Test with RPC validation enabled (skip_rpc_validation = false)
        let result = config.validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Failed to get account")));
    }

    #[tokio::test]
    async fn test_validate_with_result_skip_rpc_validation() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: EnabledMethods::default(),
                api_key: None,
                hmac_secret: None,
                max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE,
            },
            metrics: MetricsConfig::default(),
        };

        // Use account not found RPC client - should not matter when skipping RPC validation
        let rpc_client = create_mock_rpc_client_account_not_found();

        // Test with RPC validation disabled (skip_rpc_validation = true)
        let result = config.validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok()); // Should pass because RPC validation is skipped
    }
}
