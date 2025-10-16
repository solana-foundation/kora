use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use spl_token_2022::extension::ExtensionType;
use std::{fs, path::Path, str::FromStr};
use toml;
use utoipa::ToSchema;

use crate::{
    constant::{
        DEFAULT_CACHE_ACCOUNT_TTL, DEFAULT_CACHE_DEFAULT_TTL,
        DEFAULT_FEE_PAYER_BALANCE_METRICS_EXPIRY_SECONDS, DEFAULT_MAX_REQUEST_BODY_SIZE,
        DEFAULT_MAX_TIMESTAMP_AGE, DEFAULT_METRICS_ENDPOINT, DEFAULT_METRICS_PORT,
        DEFAULT_METRICS_SCRAPE_INTERVAL, DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE,
        DEFAULT_USAGE_LIMIT_MAX_TRANSACTIONS,
    },
    error::KoraError,
    fee::price::{PriceConfig, PriceModel},
    oracle::PriceSource,
    sanitize_error,
};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct FeePayerBalanceMetricsConfig {
    pub enabled: bool,
    pub expiry_seconds: u64,
}

impl Default for FeePayerBalanceMetricsConfig {
    fn default() -> Self {
        Self { enabled: false, expiry_seconds: DEFAULT_FEE_PAYER_BALANCE_METRICS_EXPIRY_SECONDS }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SplTokenConfig {
    All,
    #[serde(untagged)]
    Allowlist(Vec<String>),
}

impl Default for SplTokenConfig {
    fn default() -> Self {
        SplTokenConfig::Allowlist(vec![])
    }
}

impl<'a> IntoIterator for &'a SplTokenConfig {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            SplTokenConfig::All => [].iter(),
            SplTokenConfig::Allowlist(tokens) => tokens.iter(),
        }
    }
}

impl SplTokenConfig {
    pub fn has_token(&self, token: &str) -> bool {
        match self {
            SplTokenConfig::All => true,
            SplTokenConfig::Allowlist(tokens) => tokens.iter().any(|s| s == token),
        }
    }

    pub fn has_tokens(&self) -> bool {
        match self {
            SplTokenConfig::All => true,
            SplTokenConfig::Allowlist(tokens) => !tokens.is_empty(),
        }
    }

    pub fn as_slice(&self) -> &[String] {
        match self {
            SplTokenConfig::All => &[],
            SplTokenConfig::Allowlist(v) => v.as_slice(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationConfig {
    pub max_allowed_lamports: u64,
    pub max_signatures: u64,
    pub allowed_programs: Vec<String>,
    pub allowed_tokens: Vec<String>,
    pub allowed_spl_paid_tokens: SplTokenConfig,
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

    pub fn supports_token(&self, token: &str) -> bool {
        self.allowed_spl_paid_tokens.has_token(token)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct FeePayerPolicy {
    pub allow_sol_transfers: bool,
    pub allow_spl_transfers: bool,
    pub allow_token2022_transfers: bool,
    pub allow_assign: bool,
    pub allow_burn: bool,
    pub allow_close_account: bool,
    pub allow_approve: bool,
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

    /// Returns a Vec of enabled JSON-RPC method names
    pub fn get_enabled_method_names(&self) -> Vec<String> {
        let mut methods = Vec::new();
        if self.liveness {
            methods.push("liveness".to_string());
        }
        if self.estimate_transaction_fee {
            methods.push("estimateTransactionFee".to_string());
        }
        if self.get_supported_tokens {
            methods.push("getSupportedTokens".to_string());
        }
        if self.get_payer_signer {
            methods.push("getPayerSigner".to_string());
        }
        if self.sign_transaction {
            methods.push("signTransaction".to_string());
        }
        if self.sign_and_send_transaction {
            methods.push("signAndSendTransaction".to_string());
        }
        if self.transfer_transaction {
            methods.push("transferTransaction".to_string());
        }
        if self.get_blockhash {
            methods.push("getBlockhash".to_string());
        }
        if self.get_config {
            methods.push("getConfig".to_string());
        }
        if self.sign_transaction_if_paid {
            methods.push("signTransactionIfPaid".to_string());
        }
        methods
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

fn default_max_request_body_size() -> usize {
    DEFAULT_MAX_REQUEST_BODY_SIZE
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct KoraConfig {
    pub rate_limit: u64,
    #[serde(default = "default_max_request_body_size")]
    pub max_request_body_size: usize,
    #[serde(default)]
    pub enabled_methods: EnabledMethods,
    #[serde(default)]
    pub auth: AuthConfig,
    /// Optional payment address to receive payments (defaults to signer address)
    pub payment_address: Option<String>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub usage_limit: UsageLimitConfig,
}

impl Default for KoraConfig {
    fn default() -> Self {
        Self {
            rate_limit: 100,
            max_request_body_size: DEFAULT_MAX_REQUEST_BODY_SIZE,
            enabled_methods: EnabledMethods::default(),
            auth: AuthConfig::default(),
            payment_address: None,
            cache: CacheConfig::default(),
            usage_limit: UsageLimitConfig::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageLimitConfig {
    /// Enable per-wallet usage limiting
    pub enabled: bool,
    /// Cache URL for shared usage limiting across multiple Kora instances
    pub cache_url: Option<String>,
    /// Default maximum transactions per wallet (0 = unlimited)
    pub max_transactions: u64,
    /// Fallback behavior when cache is unavailable
    pub fallback_if_unavailable: bool,
}

impl Default for UsageLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_url: None,
            max_transactions: DEFAULT_USAGE_LIMIT_MAX_TRANSACTIONS,
            fallback_if_unavailable: DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
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
            KoraError::InternalServerError(format!(
                "Failed to read config file: {}",
                sanitize_error!(e)
            ))
        })?;

        let mut config: Config = toml::from_str(&contents).map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to parse config file: {}",
                sanitize_error!(e)
            ))
        })?;

        // Initialize Token2022Config to parse and cache extensions
        config.validation.token_2022.initialize().map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to initialize Token2022 config: {}",
                sanitize_error!(e)
            ))
        })?;

        Ok(config)
    }
}

impl KoraConfig {
    /// Get the payment address from config or fallback to signer address
    pub fn get_payment_address(&self, signer_pubkey: &Pubkey) -> Result<Pubkey, KoraError> {
        if let Some(payment_address_str) = &self.payment_address {
            let payment_address = Pubkey::from_str(payment_address_str).map_err(|_| {
                KoraError::InternalServerError("Invalid payment_address format".to_string())
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
            .with_spl_paid_tokens(SplTokenConfig::Allowlist(vec!["token3".to_string()]))
            .with_disallowed_accounts(vec!["account1"])
            .build_config()
            .unwrap();

        assert_eq!(config.validation.max_allowed_lamports, 1000000000);
        assert_eq!(config.validation.max_signatures, 10);
        assert_eq!(config.validation.allowed_programs, vec!["program1", "program2"]);
        assert_eq!(config.validation.allowed_tokens, vec!["token1", "token2"]);
        assert_eq!(
            config.validation.allowed_spl_paid_tokens,
            SplTokenConfig::Allowlist(vec!["token3".to_string()])
        );
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
            .with_spl_paid_tokens(SplTokenConfig::Allowlist(vec!["token3".to_string()]))
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
    fn test_parse_spl_payment_config() {
        let config =
            ConfigBuilder::new().with_spl_paid_tokens(SplTokenConfig::All).build_config().unwrap();

        assert_eq!(config.validation.allowed_spl_paid_tokens, SplTokenConfig::All);
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

    #[test]
    fn test_usage_limit_config_parsing() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), 10, false)
            .build_config()
            .unwrap();

        assert!(config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.cache_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.kora.usage_limit.max_transactions, 10);
        assert!(!config.kora.usage_limit.fallback_if_unavailable);
    }

    #[test]
    fn test_usage_limit_config_default() {
        let config = ConfigBuilder::new().build_config().unwrap();

        assert!(!config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.cache_url, None);
        assert_eq!(config.kora.usage_limit.max_transactions, DEFAULT_USAGE_LIMIT_MAX_TRANSACTIONS);
        assert_eq!(
            config.kora.usage_limit.fallback_if_unavailable,
            DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE
        );
    }

    #[test]
    fn test_usage_limit_config_unlimited() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, None, 0, true)
            .build_config()
            .unwrap();

        assert!(config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.max_transactions, 0); // 0 = unlimited
    }

    #[test]
    fn test_max_request_body_size_default() {
        let config = ConfigBuilder::new().build_config().unwrap();

        assert_eq!(config.kora.max_request_body_size, DEFAULT_MAX_REQUEST_BODY_SIZE);
        assert_eq!(config.kora.max_request_body_size, 2 * 1024 * 1024); // 2 MB
    }

    #[test]
    fn test_max_request_body_size_custom() {
        let custom_size = 10 * 1024 * 1024; // 10 MB
        let config =
            ConfigBuilder::new().with_max_request_body_size(custom_size).build_config().unwrap();

        assert_eq!(config.kora.max_request_body_size, custom_size);
    }
}
