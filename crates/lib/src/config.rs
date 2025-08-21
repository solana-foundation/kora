use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use spl_token_2022::extension::ExtensionType;
use std::{fs, path::Path, str::FromStr};
use toml;
use utoipa::ToSchema;

use crate::{
    constant::{
        DEFAULT_CACHE_ACCOUNT_TTL, DEFAULT_CACHE_DEFAULT_TTL,
        DEFAULT_FEE_PAYER_BALANCE_METRICS_EXPIRY_SECONDS, DEFAULT_MAX_TIMESTAMP_AGE,
        DEFAULT_METRICS_ENDPOINT, DEFAULT_METRICS_PORT, DEFAULT_METRICS_SCRAPE_INTERVAL,
    },
    error::KoraError,
    fee::price::{PriceConfig, PriceModel},
    oracle::PriceSource,
};

#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default)]
    pub fee_payer_balance: FeePayerBalanceMetricsConfig,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: DEFAULT_METRICS_ENDPOINT.to_string(),
            port: DEFAULT_METRICS_PORT,
            scrape_interval: DEFAULT_METRICS_SCRAPE_INTERVAL,
            fee_payer_balance: FeePayerBalanceMetricsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeePayerBalanceMetricsConfig {
    pub enabled: bool,
    pub expiry_seconds: u64,
}

impl Default for FeePayerBalanceMetricsConfig {
    fn default() -> Self {
        Self { enabled: false, expiry_seconds: DEFAULT_FEE_PAYER_BALANCE_METRICS_EXPIRY_SECONDS }
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
    #[serde(default)]
    pub token_2022: Token2022Config,
}

impl ValidationConfig {
    pub fn is_payment_required(&self) -> bool {
        !matches!(&self.price.model, PriceModel::Free)
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
pub struct Token2022Config {
    pub blocked_mint_extensions: Vec<String>,
    pub blocked_account_extensions: Vec<String>,
    #[serde(skip)]
    parsed_blocked_mint_extensions: Option<Vec<ExtensionType>>,
    #[serde(skip)]
    parsed_blocked_account_extensions: Option<Vec<ExtensionType>>,
}

impl Default for Token2022Config {
    fn default() -> Self {
        Self {
            blocked_mint_extensions: Vec::new(),
            blocked_account_extensions: Vec::new(),
            parsed_blocked_mint_extensions: Some(Vec::new()),
            parsed_blocked_account_extensions: Some(Vec::new()),
        }
    }
}

impl Token2022Config {
    /// Initialize and parse extension strings into ExtensionTypes
    /// This should be called after deserialization to populate the cached fields
    pub fn initialize(&mut self) -> Result<(), String> {
        let mut mint_extensions = Vec::new();
        for name in &self.blocked_mint_extensions {
            match crate::token::spl_token_2022_util::parse_mint_extension_string(name) {
                Some(ext) => {
                    mint_extensions.push(ext);
                }
                None => {
                    return Err(format!(
                        "Invalid mint extension name: '{}'. Valid names are: {:?}",
                        name,
                        crate::token::spl_token_2022_util::get_all_mint_extension_names()
                    ));
                }
            }
        }
        self.parsed_blocked_mint_extensions = Some(mint_extensions);

        let mut account_extensions = Vec::new();
        for name in &self.blocked_account_extensions {
            match crate::token::spl_token_2022_util::parse_account_extension_string(name) {
                Some(ext) => {
                    account_extensions.push(ext);
                }
                None => {
                    return Err(format!(
                        "Invalid account extension name: '{}'. Valid names are: {:?}",
                        name,
                        crate::token::spl_token_2022_util::get_all_account_extension_names()
                    ));
                }
            }
        }
        self.parsed_blocked_account_extensions = Some(account_extensions);

        Ok(())
    }

    /// Get all blocked mint extensions as ExtensionType
    pub fn get_blocked_mint_extensions(&self) -> &[ExtensionType] {
        self.parsed_blocked_mint_extensions.as_deref().unwrap_or(&[])
    }

    /// Get all blocked account extensions as ExtensionType
    pub fn get_blocked_account_extensions(&self) -> &[ExtensionType] {
        self.parsed_blocked_account_extensions.as_deref().unwrap_or(&[])
    }

    /// Check if a mint extension is blocked
    pub fn is_mint_extension_blocked(&self, ext: ExtensionType) -> bool {
        self.get_blocked_mint_extensions().contains(&ext)
    }

    /// Check if an account extension is blocked
    pub fn is_account_extension_blocked(&self, ext: ExtensionType) -> bool {
        self.get_blocked_account_extensions().contains(&ext)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnabledMethods {
    pub liveness: bool,
    pub estimate_transaction_fee: bool,
    pub get_supported_tokens: bool,
    pub get_payer_signer: bool,
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
            self.get_payer_signer,
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
    type IntoIter = std::array::IntoIter<bool, 10>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.liveness,
            self.estimate_transaction_fee,
            self.get_supported_tokens,
            self.get_payer_signer,
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
            get_payer_signer: true,
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
pub struct CacheConfig {
    /// Redis URL for caching (e.g., "redis://localhost:6379")
    pub url: Option<String>,
    /// Enable caching for RPC calls
    pub enabled: bool,
    /// Default TTL for cached entries in seconds
    pub default_ttl: u64,
    /// TTL for account data cache in seconds
    pub account_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            url: None,
            enabled: false,
            default_ttl: DEFAULT_CACHE_DEFAULT_TTL,
            account_ttl: DEFAULT_CACHE_ACCOUNT_TTL,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KoraConfig {
    pub rate_limit: u64,
    #[serde(default)]
    pub enabled_methods: EnabledMethods,
    #[serde(default)]
    pub auth: AuthConfig,
    /// Optional payment address to receive payments (defaults to signer address)
    pub payment_address: Option<String>,
    #[serde(default)]
    pub cache: CacheConfig,
}

impl Default for KoraConfig {
    fn default() -> Self {
        Self {
            rate_limit: 100,
            enabled_methods: EnabledMethods::default(),
            auth: AuthConfig::default(),
            payment_address: None,
            cache: CacheConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub hmac_secret: Option<String>,
    #[serde(default = "default_max_timestamp_age")]
    pub max_timestamp_age: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self { api_key: None, hmac_secret: None, max_timestamp_age: DEFAULT_MAX_TIMESTAMP_AGE }
    }
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
        let contents = fs::read_to_string(path).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to read config file: {e}"))
        })?;

        let mut config: Config = toml::from_str(&contents).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to parse config file: {e}"))
        })?;

        // Initialize Token2022Config to parse and cache extensions
        config.validation.token_2022.initialize().map_err(|e| {
            KoraError::InternalServerError(format!("Failed to initialize Token2022 config: {e}"))
        })?;

        Ok(config)
    }
}

impl KoraConfig {
    /// Get the payment address from config or fallback to signer address
    pub fn get_payment_address(&self, signer_pubkey: &Pubkey) -> Result<Pubkey, KoraError> {
        if let Some(payment_address_str) = &self.payment_address {
            let payment_address = Pubkey::from_str(payment_address_str).map_err(|_| {
                KoraError::InternalServerError(format!(
                    "Invalid payment_address: {payment_address_str}"
                ))
            })?;
            Ok(payment_address)
        } else {
            Ok(*signer_pubkey)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        fee::price::PriceModel,
        tests::toml_mock::{create_invalid_config, ConfigBuilder},
    };

    use super::*;

    #[test]
    fn test_load_valid_config() {
        let config = ConfigBuilder::new()
            .with_programs(vec!["program1", "program2"])
            .with_tokens(vec!["token1", "token2"])
            .with_spl_paid_tokens(vec!["token3"])
            .with_disallowed_accounts(vec!["account1"])
            .build_config()
            .unwrap();

        assert_eq!(config.validation.max_allowed_lamports, 1000000000);
        assert_eq!(config.validation.max_signatures, 10);
        assert_eq!(config.validation.allowed_programs, vec!["program1", "program2"]);
        assert_eq!(config.validation.allowed_tokens, vec!["token1", "token2"]);
        assert_eq!(config.validation.allowed_spl_paid_tokens, vec!["token3"]);
        assert_eq!(config.validation.disallowed_accounts, vec!["account1"]);
        assert_eq!(config.validation.price_source, PriceSource::Jupiter);
        assert_eq!(config.kora.rate_limit, 100);
        assert!(config.kora.enabled_methods.estimate_transaction_fee);
        assert!(config.kora.enabled_methods.sign_and_send_transaction);
    }

    #[test]
    fn test_load_config_with_enabled_methods() {
        let config = ConfigBuilder::new()
            .with_programs(vec!["program1", "program2"])
            .with_tokens(vec!["token1", "token2"])
            .with_spl_paid_tokens(vec!["token3"])
            .with_disallowed_accounts(vec!["account1"])
            .with_enabled_methods(&[
                ("liveness", true),
                ("estimate_transaction_fee", false),
                ("get_supported_tokens", true),
                ("sign_transaction", true),
                ("sign_and_send_transaction", false),
                ("transfer_transaction", true),
                ("get_blockhash", true),
                ("get_config", true),
                ("sign_transaction_if_paid", true),
                ("get_payer_signer", true),
            ])
            .build_config()
            .unwrap();

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
        let result = create_invalid_config("invalid toml content");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Config::load_config("nonexistent_file.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_margin_price_config() {
        let config = ConfigBuilder::new().with_margin_price(0.1).build_config().unwrap();

        match &config.validation.price.model {
            PriceModel::Margin { margin } => {
                assert_eq!(*margin, 0.1);
            }
            _ => panic!("Expected Margin price model"),
        }
    }

    #[test]
    fn test_parse_fixed_price_config() {
        let config = ConfigBuilder::new()
            .with_fixed_price(1000000, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU")
            .build_config()
            .unwrap();

        match &config.validation.price.model {
            PriceModel::Fixed { amount, token } => {
                assert_eq!(*amount, 1000000);
                assert_eq!(token, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
            }
            _ => panic!("Expected Fixed price model"),
        }
    }

    #[test]
    fn test_parse_free_price_config() {
        let config = ConfigBuilder::new().with_free_price().build_config().unwrap();

        match &config.validation.price.model {
            PriceModel::Free => {
                // Test passed
            }
            _ => panic!("Expected Free price model"),
        }
    }

    #[test]
    fn test_parse_missing_price_config() {
        let config = ConfigBuilder::new().build_config().unwrap();

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
        let result = ConfigBuilder::new().with_invalid_price("invalid_type").build_config();

        assert!(result.is_err());
        if let Err(KoraError::InternalServerError(msg)) = result {
            assert!(msg.contains("Failed to parse config file"));
        } else {
            panic!("Expected InternalServerError with parsing failure message");
        }
    }

    #[test]
    fn test_token2022_config_parsing() {
        let config = ConfigBuilder::new()
            .with_token2022_extensions(
                vec!["transfer_fee_config", "pausable"],
                vec!["memo_transfer", "cpi_guard"],
            )
            .build_config()
            .unwrap();

        assert_eq!(
            config.validation.token_2022.blocked_mint_extensions,
            vec!["transfer_fee_config", "pausable"]
        );
        assert_eq!(
            config.validation.token_2022.blocked_account_extensions,
            vec!["memo_transfer", "cpi_guard"]
        );

        let mint_extensions = config.validation.token_2022.get_blocked_mint_extensions();
        assert_eq!(mint_extensions.len(), 2);

        let account_extensions = config.validation.token_2022.get_blocked_account_extensions();
        assert_eq!(account_extensions.len(), 2);
    }

    #[test]
    fn test_token2022_config_invalid_extension() {
        let result = ConfigBuilder::new()
            .with_token2022_extensions(vec!["invalid_extension"], vec![])
            .build_config();

        assert!(result.is_err());
        if let Err(KoraError::InternalServerError(msg)) = result {
            assert!(msg.contains("Failed to initialize Token2022 config"));
            assert!(msg.contains("Invalid mint extension name: 'invalid_extension'"));
        } else {
            panic!("Expected InternalServerError with Token2022 initialization failure");
        }
    }

    #[test]
    fn test_token2022_config_default() {
        let config = ConfigBuilder::new().build_config().unwrap();

        assert!(config.validation.token_2022.blocked_mint_extensions.is_empty());
        assert!(config.validation.token_2022.blocked_account_extensions.is_empty());

        assert!(config.validation.token_2022.get_blocked_mint_extensions().is_empty());
        assert!(config.validation.token_2022.get_blocked_account_extensions().is_empty());
    }

    #[test]
    fn test_token2022_extension_blocking_check() {
        let config = ConfigBuilder::new()
            .with_token2022_extensions(
                vec!["transfer_fee_config", "pausable"],
                vec!["memo_transfer"],
            )
            .build_config()
            .unwrap();

        // Test mint extension blocking
        assert!(config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::TransferFeeConfig));
        assert!(config.validation.token_2022.is_mint_extension_blocked(ExtensionType::Pausable));
        assert!(!config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::NonTransferable));

        // Test account extension blocking
        assert!(config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::MemoTransfer));
        assert!(!config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::CpiGuard));
    }

    #[test]
    fn test_cache_config_parsing() {
        let config = ConfigBuilder::new()
            .with_cache_config(Some("redis://localhost:6379"), true, 600, 120)
            .build_config()
            .unwrap();

        assert_eq!(config.kora.cache.url, Some("redis://localhost:6379".to_string()));
        assert!(config.kora.cache.enabled);
        assert_eq!(config.kora.cache.default_ttl, 600);
        assert_eq!(config.kora.cache.account_ttl, 120);
    }

    #[test]
    fn test_cache_config_default() {
        let config = ConfigBuilder::new().build_config().unwrap();

        assert_eq!(config.kora.cache.url, None);
        assert!(!config.kora.cache.enabled);
        assert_eq!(config.kora.cache.default_ttl, 300);
        assert_eq!(config.kora.cache.account_ttl, 60);
    }
}
