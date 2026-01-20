use crate::{
    config::{
        AuthConfig, BundleConfig, CacheConfig, Config, EnabledMethods,
        FeePayerBalanceMetricsConfig, FeePayerPolicy, KoraConfig, MetricsConfig,
        NonceInstructionPolicy, SplTokenConfig, SplTokenInstructionPolicy, SystemInstructionPolicy,
        Token2022Config, Token2022InstructionPolicy, ValidationConfig,
    },
    constant::DEFAULT_MAX_REQUEST_BODY_SIZE,
    fee::price::PriceConfig,
    oracle::PriceSource,
    signer::config::{
        MemorySignerConfig, PrivySignerConfig, SelectionStrategy, SignerConfig, SignerPoolConfig,
        SignerPoolSettings, SignerTypeConfig, TurnkeySignerConfig, VaultSignerConfig,
    },
    usage_limit::{UsageLimitConfig, UsageLimitRuleConfig},
};
use solana_sdk::pubkey::Pubkey;

/// Mock state management for test isolation
///
/// This module provides mutex-based test isolation for config state.
pub mod mock_state {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::{Arc, Mutex, MutexGuard, RwLock};

    // Global mock config storage
    static MOCK_CONFIG: Lazy<Arc<RwLock<Option<Config>>>> =
        Lazy::new(|| Arc::new(RwLock::new(None)));

    // Mutex to synchronize access to global mock state
    static MOCK_MTX: Mutex<()> = Mutex::new(());

    /// Setup config mock with global state
    /// Returns a lock guard that should be held for the duration of the test
    pub fn setup_config_mock(config: Config) -> MutexGuard<'static, ()> {
        let lock = MOCK_MTX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        // Set the mock config globally
        let mut mock_config = MOCK_CONFIG.write().unwrap();
        *mock_config = Some(config);
        drop(mock_config);

        lock
    }

    pub fn get_config() -> Result<Config, crate::KoraError> {
        let mock_config = MOCK_CONFIG.read().unwrap();
        match &*mock_config {
            Some(config) => Ok(config.clone()),
            None => Err(crate::KoraError::InternalServerError(
                "Mock config not initialized".to_string(),
            )),
        }
    }
}

/// Primary configuration builder for test mocks
///
/// Provides a fluent interface for building Config objects with sensible defaults.
pub struct ConfigMockBuilder {
    config: Config,
}

impl Default for ConfigMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigMockBuilder {
    /// Create a new config mock builder with sensible defaults
    pub fn new() -> Self {
        Self {
            config: Config {
                validation: ValidationConfig {
                    max_allowed_lamports: 1_000_000_000,
                    max_signatures: 10,
                    allowed_programs: vec![
                        "11111111111111111111111111111111".parse().unwrap(), // System Program
                        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap(), // Token Program
                        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".parse().unwrap(), // ATA Program
                    ],
                    allowed_tokens: vec![
                        "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".parse().unwrap(), // USDC devnet
                    ],
                    allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                        "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".parse().unwrap(), // USDC devnet
                    ]),
                    disallowed_accounts: vec![],
                    price_source: PriceSource::Mock,
                    fee_payer_policy: FeePayerPolicy::default(),
                    price: PriceConfig::default(),
                    token_2022: Token2022Config::default(),
                    allow_durable_transactions: false,
                },
                kora: KoraConfig {
                    rate_limit: 100,
                    max_request_body_size: DEFAULT_MAX_REQUEST_BODY_SIZE,
                    enabled_methods: EnabledMethods::default(),
                    auth: AuthConfig::default(),
                    payment_address: None,
                    cache: CacheConfig {
                        url: Some("redis://localhost:6379".to_string()),
                        enabled: true,
                        default_ttl: 300,
                        account_ttl: 60,
                    },
                    usage_limit: UsageLimitConfig::default(),
                    bundle: BundleConfig::default(),
                },
                metrics: MetricsConfig::default(),
            },
        }
    }

    pub fn build(self) -> Config {
        self.config
    }

    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.config.validation = validation;
        self
    }

    pub fn with_kora(mut self, kora: KoraConfig) -> Self {
        self.config.kora = kora;
        self
    }

    pub fn with_metrics(mut self, metrics: MetricsConfig) -> Self {
        self.config.metrics = metrics;
        self
    }

    pub fn with_cache_enabled(mut self, enabled: bool) -> Self {
        self.config.kora.cache.enabled = enabled;
        self
    }

    pub fn with_cache_url(mut self, url: Option<String>) -> Self {
        self.config.kora.cache.url = url;
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: u64) -> Self {
        self.config.kora.rate_limit = rate_limit;
        self
    }

    pub fn with_price_source(mut self, price_source: PriceSource) -> Self {
        self.config.validation.price_source = price_source;
        self
    }

    pub fn with_allowed_programs(mut self, programs: Vec<String>) -> Self {
        self.config.validation.allowed_programs = programs;
        self
    }

    pub fn with_allowed_tokens(mut self, tokens: Vec<String>) -> Self {
        self.config.validation.allowed_tokens = tokens;
        self
    }

    pub fn with_allowed_spl_paid_tokens(mut self, spl_payment_config: SplTokenConfig) -> Self {
        self.config.validation.allowed_spl_paid_tokens = spl_payment_config;
        self
    }

    pub fn with_payment_address(mut self, payment_address: Option<String>) -> Self {
        self.config.kora.payment_address = payment_address;
        self
    }

    pub fn with_api_key_auth(mut self, api_key: String) -> Self {
        self.config.kora.auth.api_key = Some(api_key);
        self
    }

    pub fn with_hmac_auth(mut self, hmac_secret: String) -> Self {
        self.config.kora.auth.hmac_secret = Some(hmac_secret);
        self
    }

    pub fn with_max_allowed_lamports(mut self, max_lamports: u64) -> Self {
        self.config.validation.max_allowed_lamports = max_lamports;
        self
    }

    pub fn with_max_signatures(mut self, max_signatures: u64) -> Self {
        self.config.validation.max_signatures = max_signatures;
        self
    }

    pub fn with_fee_payer_policy(mut self, policy: FeePayerPolicy) -> Self {
        self.config.validation.fee_payer_policy = policy;
        self
    }

    pub fn with_usage_limit_enabled(mut self, enabled: bool) -> Self {
        self.config.kora.usage_limit.enabled = enabled;
        self
    }

    pub fn with_usage_limit_cache_url(mut self, cache_url: Option<String>) -> Self {
        self.config.kora.usage_limit.cache_url = cache_url;
        self
    }

    pub fn with_usage_limit_fallback(mut self, fallback_if_unavailable: bool) -> Self {
        self.config.kora.usage_limit.fallback_if_unavailable = fallback_if_unavailable;
        self
    }

    pub fn with_usage_limit_rules(mut self, rules: Vec<UsageLimitRuleConfig>) -> Self {
        self.config.kora.usage_limit.rules = rules;
        self
    }

    pub fn with_disallowed_accounts(mut self, accounts: Vec<String>) -> Self {
        self.config.validation.disallowed_accounts = accounts;
        self
    }

    pub fn with_bundle_enabled(mut self, enabled: bool) -> Self {
        self.config.kora.bundle.enabled = enabled;
        self
    }

    pub fn with_blocked_token2022_mint_extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.validation.token_2022.blocked_mint_extensions = extensions;
        let _ = self.config.validation.token_2022.initialize();
        self
    }

    pub fn with_blocked_token2022_account_extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.validation.token_2022.blocked_account_extensions = extensions;
        let _ = self.config.validation.token_2022.initialize();
        self
    }

    pub fn with_allow_durable_transactions(mut self, allow: bool) -> Self {
        self.config.validation.allow_durable_transactions = allow;
        self
    }

    /// Build and setup the config mock with mutex lock
    /// Returns a lock guard that should be held for the duration of the test
    pub fn build_and_setup(self) -> std::sync::MutexGuard<'static, ()> {
        mock_state::setup_config_mock(self.config)
    }
}

pub struct ValidationConfigBuilder {
    config: ValidationConfig,
}

impl Default for ValidationConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ValidationConfig {
                max_allowed_lamports: 1_000_000_000,
                max_signatures: 10,
                allowed_programs: vec![],
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
            },
        }
    }

    pub fn build(self) -> ValidationConfig {
        self.config
    }

    pub fn with_max_lamports(mut self, max_lamports: u64) -> Self {
        self.config.max_allowed_lamports = max_lamports;
        self
    }

    pub fn with_max_signatures(mut self, max_signatures: u64) -> Self {
        self.config.max_signatures = max_signatures;
        self
    }

    pub fn with_allowed_programs(mut self, programs: Vec<String>) -> Self {
        self.config.allowed_programs = programs;
        self
    }

    pub fn with_allowed_spl_paid_tokens(mut self, spl_payment_config: SplTokenConfig) -> Self {
        self.config.allowed_spl_paid_tokens = spl_payment_config;
        self
    }

    pub fn with_allowed_tokens(mut self, tokens: Vec<String>) -> Self {
        self.config.allowed_tokens = tokens;
        self
    }

    pub fn with_price_source(mut self, price_source: PriceSource) -> Self {
        self.config.price_source = price_source;
        self
    }

    pub fn with_fee_payer_policy(mut self, policy: FeePayerPolicy) -> Self {
        self.config.fee_payer_policy = policy;
        self
    }
}

pub struct KoraConfigBuilder {
    config: KoraConfig,
}

impl Default for KoraConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl KoraConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: KoraConfig {
                rate_limit: 100,
                max_request_body_size: DEFAULT_MAX_REQUEST_BODY_SIZE,
                enabled_methods: EnabledMethods::default(),
                auth: AuthConfig::default(),
                payment_address: None,
                cache: CacheConfig {
                    url: Some("redis://localhost:6379".to_string()),
                    enabled: true,
                    default_ttl: 300,
                    account_ttl: 60,
                },
                usage_limit: UsageLimitConfig::default(),
                bundle: BundleConfig::default(),
            },
        }
    }

    pub fn build(self) -> KoraConfig {
        self.config
    }

    pub fn with_rate_limit(mut self, rate_limit: u64) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    pub fn with_enabled_methods(mut self, methods: EnabledMethods) -> Self {
        self.config.enabled_methods = methods;
        self
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.config.auth = auth;
        self
    }

    pub fn with_payment_address(mut self, payment_address: Option<String>) -> Self {
        self.config.payment_address = payment_address;
        self
    }

    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.config.cache = cache;
        self
    }
}

pub struct CacheConfigBuilder {
    config: CacheConfig,
}

impl Default for CacheConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: CacheConfig {
                url: Some("redis://localhost:6379".to_string()),
                enabled: true,
                default_ttl: 300,
                account_ttl: 60,
            },
        }
    }

    pub fn build(self) -> CacheConfig {
        self.config
    }

    pub fn with_url(mut self, url: Option<String>) -> Self {
        self.config.url = url;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn with_default_ttl(mut self, ttl: u64) -> Self {
        self.config.default_ttl = ttl;
        self
    }

    pub fn with_account_ttl(mut self, ttl: u64) -> Self {
        self.config.account_ttl = ttl;
        self
    }

    pub fn disabled() -> Self {
        Self { config: CacheConfig { url: None, enabled: false, default_ttl: 0, account_ttl: 0 } }
    }
}

pub struct AuthConfigBuilder {
    config: AuthConfig,
}

impl Default for AuthConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AuthConfig {
                api_key: None,
                hmac_secret: None,
                recaptcha_secret: None,
                recaptcha_score_threshold: crate::constant::DEFAULT_RECAPTCHA_SCORE_THRESHOLD,
                max_timestamp_age: 10,
                protected_methods: crate::constant::DEFAULT_PROTECTED_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        }
    }

    pub fn build(self) -> AuthConfig {
        self.config
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.config.api_key = Some(api_key);
        self
    }

    pub fn with_hmac_secret(mut self, hmac_secret: String) -> Self {
        self.config.hmac_secret = Some(hmac_secret);
        self
    }

    pub fn with_both_auth(mut self, api_key: String, hmac_secret: String) -> Self {
        self.config.api_key = Some(api_key);
        self.config.hmac_secret = Some(hmac_secret);
        self
    }

    pub fn with_recaptcha(
        mut self,
        secret: String,
        score_threshold: Option<f64>,
        protected_methods: Option<Vec<String>>,
    ) -> Self {
        self.config.recaptcha_secret = Some(secret);
        if let Some(threshold) = score_threshold {
            self.config.recaptcha_score_threshold = threshold;
        }
        if let Some(methods) = protected_methods {
            self.config.protected_methods = methods;
        }
        self
    }
}

pub struct FeePayerPolicyBuilder {
    config: FeePayerPolicy,
}

impl Default for FeePayerPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FeePayerPolicyBuilder {
    pub fn new() -> Self {
        Self { config: FeePayerPolicy::default() }
    }

    pub fn build(self) -> FeePayerPolicy {
        self.config
    }

    pub fn with_sol_transfers(mut self, allow: bool) -> Self {
        self.config.system.allow_transfer = allow;
        self
    }

    pub fn with_spl_transfers(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_transfer = allow;
        self
    }

    pub fn with_token2022_transfers(mut self, allow: bool) -> Self {
        self.config.token_2022.allow_transfer = allow;
        self
    }

    pub fn with_assign(mut self, allow: bool) -> Self {
        self.config.system.allow_assign = allow;
        self
    }

    pub fn with_create_account(mut self, allow: bool) -> Self {
        self.config.system.allow_create_account = allow;
        self
    }

    pub fn with_allocate(mut self, allow: bool) -> Self {
        self.config.system.allow_allocate = allow;
        self
    }

    pub fn with_nonce_initialize(mut self, allow: bool) -> Self {
        self.config.system.nonce.allow_initialize = allow;
        self
    }

    pub fn with_nonce_advance(mut self, allow: bool) -> Self {
        self.config.system.nonce.allow_advance = allow;
        self
    }

    pub fn with_nonce_withdraw(mut self, allow: bool) -> Self {
        self.config.system.nonce.allow_withdraw = allow;
        self
    }

    pub fn with_nonce_authorize(mut self, allow: bool) -> Self {
        self.config.system.nonce.allow_authorize = allow;
        self
    }

    pub fn with_spl_burn(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_burn = allow;
        self.config.token_2022.allow_burn = allow;
        self
    }

    pub fn with_spl_close_account(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_close_account = allow;
        self.config.token_2022.allow_close_account = allow;
        self
    }

    pub fn with_spl_approve(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_approve = allow;
        self.config.token_2022.allow_approve = allow;
        self
    }

    pub fn with_spl_revoke(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_revoke = allow;
        self.config.token_2022.allow_revoke = allow;
        self
    }

    pub fn with_spl_set_authority(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_set_authority = allow;
        self.config.token_2022.allow_set_authority = allow;
        self
    }

    pub fn with_spl_mint_to(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_mint_to = allow;
        self.config.token_2022.allow_mint_to = allow;
        self
    }

    pub fn with_spl_freeze_account(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_freeze_account = allow;
        self.config.token_2022.allow_freeze_account = allow;
        self
    }

    pub fn with_spl_thaw_account(mut self, allow: bool) -> Self {
        self.config.spl_token.allow_thaw_account = allow;
        self.config.token_2022.allow_thaw_account = allow;
        self
    }

    pub fn restrictive() -> Self {
        Self {
            config: FeePayerPolicy {
                system: SystemInstructionPolicy {
                    allow_transfer: false,
                    allow_assign: false,
                    allow_create_account: false,
                    allow_allocate: false,
                    nonce: NonceInstructionPolicy {
                        allow_initialize: false,
                        allow_advance: false,
                        allow_withdraw: false,
                        allow_authorize: false,
                    },
                },
                spl_token: SplTokenInstructionPolicy {
                    allow_transfer: false,
                    allow_burn: false,
                    allow_close_account: false,
                    allow_approve: false,
                    allow_revoke: false,
                    allow_set_authority: false,
                    allow_mint_to: false,
                    allow_freeze_account: false,
                    allow_thaw_account: false,
                    allow_initialize_mint: false,
                    allow_initialize_account: false,
                    allow_initialize_multisig: false,
                },
                token_2022: Token2022InstructionPolicy {
                    allow_transfer: false,
                    allow_burn: false,
                    allow_close_account: false,
                    allow_approve: false,
                    allow_revoke: false,
                    allow_set_authority: false,
                    allow_mint_to: false,
                    allow_freeze_account: false,
                    allow_thaw_account: false,
                    allow_initialize_mint: false,
                    allow_initialize_account: false,
                    allow_initialize_multisig: false,
                },
            },
        }
    }

    pub fn permissive() -> Self {
        Self { config: FeePayerPolicy::default() }
    }
}

pub struct MetricsConfigBuilder {
    config: MetricsConfig,
}

impl Default for MetricsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsConfigBuilder {
    pub fn new() -> Self {
        Self { config: MetricsConfig::default() }
    }

    pub fn build(self) -> MetricsConfig {
        self.config
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.config.endpoint = endpoint;
        self
    }

    pub fn with_fee_payer_balance(mut self, config: FeePayerBalanceMetricsConfig) -> Self {
        self.config.fee_payer_balance = config;
        self
    }

    pub fn enabled_with_endpoint(endpoint: String) -> Self {
        Self {
            config: MetricsConfig {
                enabled: true,
                port: 9464,
                scrape_interval: 10,
                endpoint,
                fee_payer_balance: FeePayerBalanceMetricsConfig::default(),
            },
        }
    }
}

pub struct SignerPoolConfigBuilder {
    config: SignerPoolConfig,
}

impl Default for SignerPoolConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SignerPoolConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SignerPoolConfig {
                signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
                signers: vec![],
            },
        }
    }

    pub fn build(self) -> SignerPoolConfig {
        self.config
    }

    pub fn with_strategy(mut self, strategy: SelectionStrategy) -> Self {
        self.config.signer_pool.strategy = strategy;
        self
    }

    pub fn with_signers(mut self, signers: Vec<SignerConfig>) -> Self {
        self.config.signers = signers;
        self
    }

    pub fn add_signer(mut self, signer: SignerConfig) -> Self {
        self.config.signers.push(signer);
        self
    }

    pub fn with_memory_signer(
        mut self,
        name: String,
        private_key_env: String,
        weight: Option<u32>,
    ) -> Self {
        let signer = SignerConfig {
            name,
            weight,
            config: SignerTypeConfig::Memory { config: MemorySignerConfig { private_key_env } },
        };
        self.config.signers.push(signer);
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_turnkey_signer(
        mut self,
        name: String,
        api_public_key_env: String,
        api_private_key_env: String,
        organization_id_env: String,
        private_key_id_env: String,
        public_key_env: String,
        weight: Option<u32>,
    ) -> Self {
        let signer = SignerConfig {
            name,
            weight,
            config: SignerTypeConfig::Turnkey {
                config: TurnkeySignerConfig {
                    api_public_key_env,
                    api_private_key_env,
                    organization_id_env,
                    private_key_id_env,
                    public_key_env,
                },
            },
        };
        self.config.signers.push(signer);
        self
    }

    pub fn with_privy_signer(
        mut self,
        name: String,
        app_id_env: String,
        app_secret_env: String,
        wallet_id_env: String,
        weight: Option<u32>,
    ) -> Self {
        let signer = SignerConfig {
            name,
            weight,
            config: SignerTypeConfig::Privy {
                config: PrivySignerConfig { app_id_env, app_secret_env, wallet_id_env },
            },
        };
        self.config.signers.push(signer);
        self
    }

    pub fn with_vault_signer(
        mut self,
        name: String,
        addr_env: String,
        token_env: String,
        key_name_env: String,
        pubkey_env: String,
        weight: Option<u32>,
    ) -> Self {
        let signer = SignerConfig {
            name,
            weight,
            config: SignerTypeConfig::Vault {
                config: VaultSignerConfig {
                    vault_addr_env: addr_env,
                    vault_token_env: token_env,
                    key_name_env,
                    pubkey_env,
                },
            },
        };
        self.config.signers.push(signer);
        self
    }
}

pub fn get_default_config() -> Config {
    ConfigMockBuilder::new().build()
}

pub fn get_default_config_with_cache() -> Config {
    ConfigMockBuilder::new()
        .with_cache_enabled(true)
        .with_cache_url(Some("redis://localhost:6379".to_string()))
        .build()
}

pub fn get_default_config_with_auth() -> Config {
    ConfigMockBuilder::new()
        .with_api_key_auth("test-api-key".to_string())
        .with_hmac_auth("test-hmac-secret".to_string())
        .build()
}

pub fn get_default_config_with_payment_address() -> Config {
    ConfigMockBuilder::new().with_payment_address(Some(Pubkey::new_unique().to_string())).build()
}

pub fn get_default_config_restrictive_policy() -> Config {
    ConfigMockBuilder::new()
        .with_validation(
            ValidationConfigBuilder::new()
                .with_fee_payer_policy(FeePayerPolicyBuilder::restrictive().build())
                .build(),
        )
        .build()
}

pub fn get_default_config_with_metrics() -> Config {
    ConfigMockBuilder::new()
        .with_metrics(
            MetricsConfigBuilder::enabled_with_endpoint(
                "http://localhost:8080/metrics".to_string(),
            )
            .build(),
        )
        .build()
}

pub fn get_default_signer_pool_config() -> SignerPoolConfig {
    SignerPoolConfigBuilder::new()
        .with_memory_signer("test_signer".to_string(), "TEST_PRIVATE_KEY".to_string(), Some(1))
        .build()
}

pub fn get_default_multi_signer_pool_config() -> SignerPoolConfig {
    SignerPoolConfigBuilder::new()
        .with_strategy(SelectionStrategy::RoundRobin)
        .with_memory_signer("memory_signer".to_string(), "MEMORY_PRIVATE_KEY".to_string(), Some(1))
        .with_turnkey_signer(
            "turnkey_signer".to_string(),
            "TURNKEY_API_PUBLIC_KEY".to_string(),
            "TURNKEY_API_PRIVATE_KEY".to_string(),
            "TURNKEY_ORG_ID".to_string(),
            "TURNKEY_PRIVATE_KEY_ID".to_string(),
            "TURNKEY_PUBLIC_KEY".to_string(),
            Some(2),
        )
        .build()
}

pub fn get_default_weighted_signer_pool_config() -> SignerPoolConfig {
    SignerPoolConfigBuilder::new()
        .with_strategy(SelectionStrategy::Weighted)
        .with_memory_signer("low_weight".to_string(), "LOW_WEIGHT_KEY".to_string(), Some(1))
        .with_memory_signer("high_weight".to_string(), "HIGH_WEIGHT_KEY".to_string(), Some(5))
        .build()
}
