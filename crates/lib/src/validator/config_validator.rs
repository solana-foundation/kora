use std::{path::Path, str::FromStr};

use crate::{
    signer::SignerPoolConfig, state::get_config, token::token::TokenUtil,
    validator::signer_validator::SignerValidator, KoraError,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

mod fee_payer_policy;
mod onchain;
mod server;
mod tokens;
mod transactions;

pub struct ConfigValidator {}

impl ConfigValidator {
    pub async fn validate(_rpc_client: &RpcClient) -> Result<(), KoraError> {
        let config = &get_config()?;

        if config.validation.is_payment_required() && config.validation.allowed_tokens.is_empty() {
            return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
        }

        if !config.validation.allowed_tokens.is_empty() {
            TokenUtil::check_valid_tokens(&config.validation.allowed_tokens)?;
        }

        if let Some(payment_address) = &config.kora.payment_address {
            if let Err(e) = Pubkey::from_str(payment_address) {
                return Err(KoraError::InternalServerError(format!(
                    "Invalid payment address: {e}"
                )));
            }
        }

        Ok(())
    }

    pub async fn validate_with_result(
        rpc_client: &RpcClient,
        skip_rpc_validation: bool,
    ) -> Result<Vec<String>, Vec<String>> {
        Self::validate_with_result_and_signers(rpc_client, skip_rpc_validation, None::<&Path>).await
    }
}

impl ConfigValidator {
    pub async fn validate_with_result_and_signers<P: AsRef<Path>>(
        rpc_client: &RpcClient,
        skip_rpc_validation: bool,
        signers_config_path: Option<P>,
    ) -> Result<Vec<String>, Vec<String>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let config = match get_config() {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("Failed to get config: {e}"));
                return Err(errors);
            }
        };

        Self::check_server(config, &mut errors, &mut warnings);
        Self::check_transaction_settings(config, &mut errors, &mut warnings);
        Self::check_programs(config, &mut errors, &mut warnings);
        Self::check_tokens(config, &mut errors, &mut warnings);
        Self::check_bundle(config, &mut errors);
        Self::validate_fee_payer_policy(&config.validation.fee_payer_policy, &mut warnings);
        Self::warn_mutable_transfer_hook_payment_risk(&config.validation, &mut warnings);
        Self::check_price_model(config, &mut errors, &mut warnings);
        Self::check_auth(config, &mut warnings);
        Self::check_caches(config, &mut errors, &mut warnings).await;

        if !skip_rpc_validation {
            Self::check_onchain(config, rpc_client, &mut errors, &mut warnings).await;
        }

        if config.validation.cross_cluster_check {
            Self::check_cross_cluster(config, rpc_client, &mut warnings).await;
        }

        if let Some(path) = signers_config_path {
            match SignerPoolConfig::load_config(path.as_ref()) {
                Ok(signer_config) => {
                    let (signer_warnings, signer_errors) =
                        SignerValidator::validate_with_result(&signer_config);
                    warnings.extend(signer_warnings);
                    errors.extend(signer_errors);
                }
                Err(e) => {
                    errors.push(format!("Failed to load signers config: {e}"));
                }
            }
        } else {
            println!("ℹ️  Signers configuration not validated. Include --signers-config path/to/signers.toml to validate signers");
        }

        println!("=== Configuration Validation ===");
        if errors.is_empty() {
            println!("✓ Configuration validation successful!");
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
    use solana_transaction::versioned::TransactionVersion;

    use crate::{
        config::{
            default_allowed_transaction_versions, AuthConfig, BundleConfig, CacheConfig, Config,
            EnabledMethods, FeePayerPolicy, KoraConfig, LighthouseConfig, MetricsConfig,
            NonceInstructionPolicy, PluginsConfig, ProgramsConfig, SplTokenConfig,
            SplTokenInstructionPolicy, SystemInstructionPolicy, Token2022Config,
            Token2022InstructionPolicy, TransactionPluginType, TransferHookPolicy,
            UsageLimitConfig, ValidationConfig, CORS_WILDCARD,
        },
        constant::{
            BPF_LOADER_UPGRADEABLE_PROGRAM_ID, DEFAULT_MAX_REQUEST_BODY_SIZE, LIGHTHOUSE_PROGRAM_ID,
        },
        fee::price::{PriceConfig, PriceModel},
        oracle::PriceSource,
        state::update_config,
        tests::{
            account_mock::create_mock_token2022_mint_with_extensions,
            common::{
                create_mock_non_executable_account, create_mock_program_account,
                create_mock_rpc_client_account_not_found, create_mock_rpc_client_with_account,
                create_mock_rpc_client_with_mint, RpcMockBuilder,
            },
            config_mock::ConfigMockBuilder,
        },
    };
    use serial_test::serial;
    use solana_commitment_config::CommitmentConfig;
    use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
    use spl_token_2022_interface::{extension::ExtensionType, ID as TOKEN_2022_PROGRAM_ID};
    use spl_token_interface::ID as SPL_TOKEN_PROGRAM_ID;

    use super::{tokens::validate_token2022_extensions, *};

    #[tokio::test]
    #[serial]
    async fn test_validate_config() {
        let mut config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1000000000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec!["program1".to_string()]),
                allowed_tokens: vec!["token1".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec!["token3".to_string()]),
                disallowed_accounts: vec!["account1".to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config.clone());

        config.validation.allowed_tokens = vec![];
        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate(&rpc_client).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InternalServerError(_)));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_successful_config() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        // Token2022 is not enabled, so only auth warning should remain.
        assert_eq!(warnings.len(), 1);
        assert!(!warnings.iter().any(|w| w.contains("PermanentDelegate")));
        assert!(warnings.iter().any(|w| w.contains("No authentication configured")));
    }

    fn validation_config_with_auth() -> ValidationConfig {
        ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_priority_fee_lamports: None,
            max_signatures: 10,
            allowed_programs: ProgramsConfig::Allowlist(vec![
                SYSTEM_PROGRAM_ID.to_string(),
                SPL_TOKEN_PROGRAM_ID.to_string(),
            ]),
            allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
            allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
            ]),
            disallowed_accounts: vec![],
            price_source: PriceSource::Jupiter,
            fee_payer_policy: FeePayerPolicy::default(),
            price: PriceConfig::default(),
            token_2022: Token2022Config::default(),
            allow_durable_transactions: false,
            allowed_transaction_versions: default_allowed_transaction_versions(),
            max_price_staleness_slots: 0,
            require_one_of_programs: vec![],
            cross_cluster_check: false,
            cross_cluster_endpoints: vec![],
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_warns_when_env_overrides_config_auth_secret() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        std::env::set_var(AuthConfig::API_KEY_ENV, "stale-env-key");
        let config = Config {
            validation: validation_config_with_auth(),
            kora: KoraConfig {
                auth: AuthConfig {
                    api_keys: Some(vec!["rotated-config-key".to_string()]),
                    ..Default::default()
                },
                ..KoraConfig::default()
            },
            metrics: MetricsConfig::default(),
        };
        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        std::env::remove_var(AuthConfig::API_KEY_ENV);
        std::env::remove_var("JUPITER_API_KEY");

        let warnings = result.expect("validation should succeed with warnings");
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("KORA_API_KEY") && w.contains("[kora.auth].api_key")),
            "expected an env-override warning naming the field, got: {warnings:?}"
        );
        // Auth is in effect, so the no-auth warning must not appear.
        assert!(!warnings.iter().any(|w| w.contains("No authentication configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_no_false_no_auth_warning_when_env_provides_auth() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        std::env::set_var(AuthConfig::API_KEY_ENV, "env-only-key");
        let config = Config {
            validation: validation_config_with_auth(),
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        std::env::remove_var(AuthConfig::API_KEY_ENV);
        std::env::remove_var("JUPITER_API_KEY");

        let warnings = result.expect("validation should succeed");
        assert!(
            !warnings.iter().any(|w| w.contains("No authentication configured")),
            "env-provided auth must suppress the no-auth warning, got: {warnings:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_rejects_usage_limit_enabled_without_rules() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                usage_limit: UsageLimitConfig {
                    enabled: true,
                    cache_url: None,
                    fallback_if_unavailable: true,
                    rules: vec![],
                },
                ..KoraConfig::default()
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        let errors = result.expect_err("enabled usage_limit with no rules must fail validation");
        assert!(errors.iter().any(|e| e.contains("no rules are configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_allows_usage_limit_disabled_without_rules() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                usage_limit: UsageLimitConfig {
                    enabled: false,
                    cache_url: None,
                    fallback_if_unavailable: true,
                    rules: vec![],
                },
                ..KoraConfig::default()
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        let warnings = result.expect("disabled usage_limit with no rules must pass validation");
        assert!(!warnings.iter().any(|w| w.contains("no rules are configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_warns_for_unblocked_permanent_delegate_when_token2022_enabled(
    ) {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                    TOKEN_2022_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(warnings.iter().any(|w| w.contains("PermanentDelegate")));
        assert!(warnings.iter().any(|w| w.contains("No authentication configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_no_permanent_delegate_warning_when_blocked() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                    TOKEN_2022_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: {
                    let mut token_2022 = Token2022Config::default();
                    token_2022.blocked_mint_extensions = vec!["permanent_delegate".to_string()];
                    token_2022
                },
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(!warnings.iter().any(|w| w.contains("PermanentDelegate")));
        assert!(warnings.iter().any(|w| w.contains("No authentication configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_warnings() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 0,
                max_priority_fee_lamports: None,
                max_signatures: 0,
                allowed_programs: ProgramsConfig::Allowlist(vec![]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                rate_limit: 0,
                global_rate_limit: Some(0),
                cors_allow_origins: vec![],
                max_request_body_size: DEFAULT_MAX_REQUEST_BODY_SIZE,
                enabled_methods: EnabledMethods {
                    liveness: false,
                    estimate_transaction_fee: false,
                    get_supported_tokens: false,
                    sign_transaction: false,
                    sign_and_send_transaction: false,
                    transfer_transaction: false,
                    get_blockhash: false,
                    get_config: false,
                    get_payer_signer: false,
                    get_version: false,
                    estimate_bundle_fee: false,
                    sign_and_send_bundle: false,
                    sign_bundle: false,
                },
                auth: AuthConfig::default(),
                payment_address: None,
                cache: CacheConfig::default(),
                usage_limit: UsageLimitConfig::default(),
                plugins: PluginsConfig::default(),
                bundle: BundleConfig::default(),
                lighthouse: LighthouseConfig::default(),
                force_sig_verify: false,
                sign_timeout_seconds: 10,
                sign_max_retries: 2,
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config.clone());

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Rate limit is set to 0")));
        assert!(warnings.iter().any(|w| w.contains("Global rate limit is set to 0")));
        assert!(warnings.iter().any(|w| w
            .contains("cors_allow_origins is empty - all cross-origin requests will be blocked")));
        assert!(warnings.iter().any(|w| w.contains("All rpc methods are disabled")));
        assert!(warnings.iter().any(|w| w.contains("Max allowed lamports is 0")));
        assert!(warnings.iter().any(|w| w.contains("Max signatures is 0")));
        assert!(warnings.iter().any(|w| w.contains("Using Mock price source")));
        assert!(warnings.iter().any(|w| w.contains("No allowed programs configured")));

        // Test partially invalid CORS origins
        let mut config_cors = config.clone();
        config_cors.kora.cors_allow_origins =
            vec!["https://valid.com".to_string(), "invalid\norigin".to_string()];
        let _ = update_config(config_cors);

        let result_cors = ConfigValidator::validate_with_result(&rpc_client, true).await;
        let warnings_cors = result_cors.unwrap();
        assert!(warnings_cors.iter().any(|w| w.contains(
            "cors_allow_origins contains 1 invalid origin(s) that will be silently filtered out"
        )));

        // Test wildcard with redundant specific origin(s)
        let mut config_cors_wildcard = config.clone();
        config_cors_wildcard.kora.cors_allow_origins =
            vec![CORS_WILDCARD.to_string(), "https://redundant.com".to_string()];
        let _ = update_config(config_cors_wildcard);

        let result_cors_wildcard = ConfigValidator::validate_with_result(&rpc_client, true).await;
        let warnings_cors_wildcard = result_cors_wildcard.unwrap();
        assert!(warnings_cors_wildcard
            .iter()
            .any(|w| w.contains("cors_allow_origins contains '*' alongside specific origin(s)")));

        // Test all invalid CORS origins
        let mut config_cors_all_invalid = config.clone();
        config_cors_all_invalid.kora.cors_allow_origins = vec![
            "invalid\n1".to_string(),
            "https://your-app.com/".to_string(),
            "https://example.com:badport".to_string(),
            "https://user:pass@example.com".to_string(),
            "https://[not-ipv6]".to_string(),
        ];
        let _ = update_config(config_cors_all_invalid);

        let result_cors_all_invalid =
            ConfigValidator::validate_with_result(&rpc_client, true).await;
        let warnings_cors_all_invalid = result_cors_all_invalid.unwrap();
        assert!(warnings_cors_all_invalid
            .iter()
            .any(|w| w.contains("cors_allow_origins contains no valid origin(s) (must be e.g., 'https://your-app.com') - all cross-origin requests will be blocked")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_duplicate_plugins_warn() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.kora.plugins.enabled =
            vec![TransactionPluginType::GasSwap, TransactionPluginType::GasSwap];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings.iter().any(|w| w.contains("Duplicate transaction plugin configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_gas_swap_plugin_requires_system_program() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        config.validation.allowed_programs =
            ProgramsConfig::Allowlist(vec![SPL_TOKEN_PROGRAM_ID.to_string()]);

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("GasSwap plugin requires System Program")));
    }

    #[tokio::test]
    #[serial]
    async fn test_gas_swap_plugin_requires_token_program() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        config.validation.allowed_programs =
            ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]);

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("GasSwap plugin requires at least one token program")));
    }

    #[tokio::test]
    #[serial]
    async fn test_gas_swap_plugin_requires_allowed_tokens() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        config.validation.allowed_tokens = vec![];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("GasSwap plugin requires at least one token in allowed_tokens")));
    }

    #[tokio::test]
    #[serial]
    async fn test_gas_swap_plugin_rejects_free_pricing() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        config.validation.price = PriceConfig { model: PriceModel::Free };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("GasSwap plugin cannot be used with Free pricing")));
    }

    #[test]
    fn test_check_transaction_settings_empty_allowed_transaction_versions_errors() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_transaction_versions = vec![];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        ConfigValidator::check_transaction_settings(&config, &mut errors, &mut warnings);

        assert!(errors.iter().any(|e| e.contains("allowed_transaction_versions is empty")));
    }

    #[test]
    fn test_check_transaction_settings_unsupported_transaction_version_errors() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_transaction_versions =
            vec![TransactionVersion::LEGACY, TransactionVersion::Number(2)];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        ConfigValidator::check_transaction_settings(&config, &mut errors, &mut warnings);

        assert!(errors
            .iter()
            .any(|e| e.contains("allowed_transaction_versions contains unsupported version 2")));
    }

    #[test]
    fn test_check_transaction_settings_single_allowed_transaction_version_ok() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_transaction_versions = vec![TransactionVersion::Number(1)];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        ConfigValidator::check_transaction_settings(&config, &mut errors, &mut warnings);

        assert!(
            !errors.iter().any(|e| e.contains("allowed_transaction_versions")),
            "a single allowed version is a valid policy: {errors:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_empty_allowed_tokens_ok_when_free() {
        // Free-pricing operators (e.g. devnet-deploy paymaster) have no reason to maintain
        // an allowed_tokens list since no SPL token is ever used for payment.
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok(), "Free pricing with empty allowed_tokens must not error");
        let warnings = result.unwrap();
        assert!(!warnings.iter().any(|w| w.contains("No allowed tokens configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_empty_allowed_tokens_errors_when_fees_required() {
        // When the price model charges fees, allowed_tokens still must be populated.
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("No allowed tokens configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_missing_system_program_warning() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    "11111111111111111111111111111112".to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings.iter().any(|w| w.contains("Missing System Program in allowed programs")));
        assert!(warnings.iter().any(|w| w.contains("Missing Token Program in allowed programs")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_invalid_must_call_program_pubkey() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.validation.require_one_of_programs = vec!["not-a-pubkey".to_string()];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.contains("Invalid base58 pubkey format in require_one_of_programs")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_must_call_program_not_in_allowed_programs() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        let required_program = solana_sdk::pubkey::Pubkey::new_unique().to_string();
        config.validation.require_one_of_programs = vec![required_program.clone()];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| {
            e.contains("require_one_of_programs must also be in allowed_programs")
                && e.contains(&required_program)
        }));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_require_one_of_programs_allows_compute_budget_program() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        let compute_budget_program = solana_compute_budget_interface::id().to_string();
        config.validation.allowed_programs = ProgramsConfig::Allowlist(vec![
            SYSTEM_PROGRAM_ID.to_string(),
            SPL_TOKEN_PROGRAM_ID.to_string(),
            compute_budget_program.clone(),
        ]);
        config.validation.require_one_of_programs = vec![compute_budget_program];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_valid_require_one_of_programs() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.validation.require_one_of_programs = vec![SYSTEM_PROGRAM_ID.to_string()];

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "invalid_token_address".to_string()
                ]),
                disallowed_accounts: vec!["invalid_account_address".to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: -0.1 } },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("No allowed tokens configured")));
        assert!(errors.iter().any(|e| e.contains("Invalid spl paid token address")));
        assert!(errors.iter().any(|e| e.contains("Invalid disallowed account address")));
        assert!(errors.iter().any(|e| e.contains("Margin cannot be negative")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_fixed_price_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0,
                        token: "invalid_token_address".to_string(),
                        strict: false,
                    },
                },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("Invalid token address for fixed price")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_fixed_price_not_in_allowed_tokens() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 1000,
                        token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                        strict: false,
                    },
                },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
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
    #[serial]
    async fn test_validate_with_result_fixed_price_zero_amount_warning() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0,
                        token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                        strict: false,
                    },
                },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings
            .iter()
            .any(|w| w.contains("Fixed price amount is 0 - transactions will be free")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_fee_validation_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("When fees are enabled, at least one token program (SPL Token or Token2022) must be in allowed_programs")));
        assert!(errors
            .iter()
            .any(|e| e.contains("When fees are enabled, allowed_spl_paid_tokens cannot be empty")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_fee_and_any_spl_token_allowed() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::All,
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();

        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());

        let warnings = result.unwrap();
        assert!(warnings.iter().any(|w| w.contains("Using 'All' for allowed_spl_paid_tokens")));
        assert!(warnings.iter().any(|w| w.contains("volatility risk")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_paid_tokens_not_in_allowed_tokens() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors.iter().any(|e| e.contains("Token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v in allowed_spl_paid_tokens must also be in allowed_tokens")));
    }

    fn create_program_only_config() -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        }
    }

    fn create_token_only_config() -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_valid_program() {
        let config = create_program_only_config();

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_account(&create_mock_program_account());

        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Token")
            && e.contains("validation failed")
            && e.contains("not found")));
        assert!(!errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_valid_token_mint() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = create_token_only_config();

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_mint(6);

        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(warnings.iter().any(|w| w.contains("No allowed programs configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_non_executable_program_fails() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_account(&create_mock_non_executable_account());

        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_account_not_found_fails() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_account_not_found();

        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 2, "Should have validation errors for programs and tokens");
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_skip_rpc_validation() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_account_not_found();

        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_valid_token2022_extensions() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: {
                    let mut config = Token2022Config::default();
                    config.blocked_mint_extensions =
                        vec!["transfer_fee_config".to_string(), "pausable".to_string()];
                    config.blocked_account_extensions =
                        vec!["memo_transfer".to_string(), "cpi_guard".to_string()];
                    config
                },
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_invalid_token2022_mint_extension() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: {
                    let mut config = Token2022Config::default();
                    config.blocked_mint_extensions = vec!["invalid_mint_extension".to_string()];
                    config
                },
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Token2022 extension validation failed")
            && e.contains("Invalid mint extension name: 'invalid_mint_extension'")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_invalid_token2022_account_extension() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![SYSTEM_PROGRAM_ID.to_string()]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: {
                    let mut config = Token2022Config::default();
                    config.blocked_account_extensions =
                        vec!["invalid_account_extension".to_string()];
                    config
                },
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Token2022 extension validation failed")
            && e.contains("Invalid account extension name: 'invalid_account_extension'")));
    }

    #[test]
    fn test_validate_token2022_extensions_valid() {
        let mut config = Token2022Config::default();
        config.blocked_mint_extensions =
            vec!["transfer_fee_config".to_string(), "pausable".to_string()];
        config.blocked_account_extensions =
            vec!["memo_transfer".to_string(), "cpi_guard".to_string()];
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_ok());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_token2022_extensions_invalid_mint_extension() {
        let mut config = Token2022Config::default();
        config.blocked_mint_extensions = vec!["invalid_extension".to_string()];
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid mint extension name: 'invalid_extension'"));
    }

    #[test]
    fn test_validate_token2022_extensions_invalid_account_extension() {
        let mut config = Token2022Config::default();
        config.blocked_account_extensions = vec!["invalid_extension".to_string()];
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid account extension name: 'invalid_extension'"));
    }

    #[test]
    fn test_validate_token2022_extensions_empty() {
        let config = Token2022Config::default();
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_ok());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_token2022_extensions_warns_for_allow_all_policy() {
        let mut config = Token2022Config::default();
        config.transfer_hook_policy = TransferHookPolicy::AllowAll;
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_ok());
        assert!(warnings.iter().any(|w| w.contains("transfer_hook_policy is 'allow_all'")));
    }

    #[test]
    fn test_validate_token2022_extensions_warns_for_deny_mutable_delayed_policy() {
        let mut config = Token2022Config::default();
        config.transfer_hook_policy = TransferHookPolicy::DenyMutableForDelayedSigning;
        let mut warnings = Vec::new();

        let result = validate_token2022_extensions(&config, &mut warnings);
        assert!(result.is_ok());
        assert!(warnings
            .iter()
            .any(|w| w.contains("transfer_hook_policy is 'deny_mutable_for_delayed_signing'")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_fee_payer_policy_warnings() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                    TOKEN_2022_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy {
                    system: SystemInstructionPolicy {
                        allow_transfer: true,
                        allow_assign: true,
                        allow_create_account: true,
                        allow_allocate: true,
                        nonce: NonceInstructionPolicy {
                            allow_initialize: true,
                            allow_advance: true,
                            allow_withdraw: true,
                            allow_authorize: true,
                        },
                    },
                    spl_token: SplTokenInstructionPolicy {
                        allow_transfer: true,
                        allow_burn: true,
                        allow_close_account: true,
                        allow_approve: true,
                        allow_revoke: true,
                        allow_set_authority: true,
                        allow_mint_to: true,
                        allow_initialize_mint: true,
                        allow_initialize_account: true,
                        allow_initialize_multisig: true,
                        allow_freeze_account: true,
                        allow_thaw_account: true,
                        allow_withdraw_excess_lamports: true,
                        allow_unwrap_lamports: true,
                    },
                    token_2022: Token2022InstructionPolicy {
                        allow_transfer: true,
                        allow_burn: true,
                        allow_close_account: true,
                        allow_approve: true,
                        allow_revoke: true,
                        allow_set_authority: true,
                        allow_mint_to: true,
                        allow_initialize_mint: true,
                        allow_initialize_account: true,
                        allow_initialize_multisig: true,
                        allow_initialize_extension_authority: true,
                        allow_update_extension_authority: true,
                        allow_freeze_account: true,
                        allow_thaw_account: true,
                        allow_withdraw_excess_lamports: true,
                        allow_unwrap_lamports: true,
                    },
                    alt: crate::config::AltInstructionPolicy {
                        allow_create: true,
                        allow_extend: true,
                        allow_freeze: true,
                        allow_deactivate: true,
                        allow_close: true,
                    },
                    bpf_loader_upgradeable: crate::config::BpfLoaderUpgradeableInstructionPolicy {
                        allow_initialize_buffer: true,
                        allow_write: true,
                        allow_deploy_with_max_data_len: true,
                        allow_upgrade: true,
                        allow_set_authority: true,
                        allow_set_authority_checked: true,
                        allow_close: true,
                        allow_extend_program: true,
                        allow_extend_program_checked: true,
                        allow_migrate: true,
                    },
                    loader_v4: crate::config::LoaderV4InstructionPolicy {
                        allow_write: true,
                        allow_copy: true,
                        allow_set_program_length: true,
                        allow_deploy: true,
                        allow_retract: true,
                        allow_transfer_authority: true,
                        allow_finalize: true,
                    },
                },
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config.clone());

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings
            .iter()
            .any(|w| w.contains("System transfers") && w.contains("allow_transfer")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("System Assign instructions") && w.contains("allow_assign")));
        assert!(warnings.iter().any(|w| w.contains("System CreateAccount instructions")
            && w.contains("allow_create_account")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("System Allocate instructions") && w.contains("allow_allocate")));

        assert!(warnings
            .iter()
            .any(|w| w.contains("nonce account initialization") && w.contains("allow_initialize")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("nonce account advancement") && w.contains("allow_advance")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("nonce account withdrawals") && w.contains("allow_withdraw")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("nonce authority changes") && w.contains("allow_authorize")));

        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token transfers") && w.contains("allow_transfer")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token burn operations") && w.contains("allow_burn")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token CloseAccount") && w.contains("allow_close_account")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token approve") && w.contains("allow_approve")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token revoke") && w.contains("allow_revoke")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token SetAuthority") && w.contains("allow_set_authority")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token MintTo") && w.contains("allow_mint_to")));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("SPL Token InitializeMint")
                    && w.contains("allow_initialize_mint"))
        );
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token InitializeAccount")
                && w.contains("allow_initialize_account")));
        assert!(warnings.iter().any(|w| w.contains("SPL Token InitializeMultisig")
            && w.contains("allow_initialize_multisig")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token FreezeAccount") && w.contains("allow_freeze_account")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("SPL Token ThawAccount") && w.contains("allow_thaw_account")));

        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 transfers") && w.contains("allow_transfer")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 burn operations") && w.contains("allow_burn")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 CloseAccount") && w.contains("allow_close_account")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 approve") && w.contains("allow_approve")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 revoke") && w.contains("allow_revoke")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 SetAuthority") && w.contains("allow_set_authority")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 MintTo") && w.contains("allow_mint_to")));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("Token2022 InitializeMint")
                    && w.contains("allow_initialize_mint"))
        );
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 InitializeAccount")
                && w.contains("allow_initialize_account")));
        assert!(warnings.iter().any(|w| w.contains("Token2022 InitializeMultisig")
            && w.contains("allow_initialize_multisig")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 FreezeAccount") && w.contains("allow_freeze_account")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Token2022 ThawAccount") && w.contains("allow_thaw_account")));

        assert!(warnings
            .iter()
            .any(|w| w.contains("ALT CreateLookupTable") && w.contains("allow_create")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("ALT ExtendLookupTable") && w.contains("allow_extend")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("ALT FreezeLookupTable") && w.contains("allow_freeze")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("ALT DeactivateLookupTable") && w.contains("allow_deactivate")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("ALT CloseLookupTable") && w.contains("allow_close")));

        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 Write") && w.contains("allow_write")));
        assert!(warnings.iter().any(|w| w.contains("Loader-v4 Copy") && w.contains("allow_copy")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 SetProgramLength")
                && w.contains("allow_set_program_length")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 Deploy") && w.contains("allow_deploy")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 Retract") && w.contains("allow_retract")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 TransferAuthority")
                && w.contains("allow_transfer_authority")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Loader-v4 Finalize") && w.contains("allow_finalize")));

        for (description, field) in [
            ("BPF Loader Upgradeable InitializeBuffer", "allow_initialize_buffer"),
            ("BPF Loader Upgradeable Write", "allow_write"),
            ("BPF Loader Upgradeable DeployWithMaxDataLen", "allow_deploy_with_max_data_len"),
            ("BPF Loader Upgradeable Upgrade", "allow_upgrade"),
            ("BPF Loader Upgradeable SetAuthority", "allow_set_authority"),
            ("BPF Loader Upgradeable SetAuthorityChecked", "allow_set_authority_checked"),
            ("BPF Loader Upgradeable Close", "allow_close"),
            ("BPF Loader Upgradeable ExtendProgram", "allow_extend_program"),
            ("BPF Loader Upgradeable ExtendProgramChecked", "allow_extend_program_checked"),
            ("BPF Loader Upgradeable Migrate", "allow_migrate"),
        ] {
            assert!(
                warnings.iter().any(|w| w.contains(description) && w.contains(field)),
                "missing warning for {description} ({field})"
            );
        }

        let fee_payer_warnings: Vec<_> =
            warnings.iter().filter(|w| w.contains("Fee payer policy")).collect();
        for warning in fee_payer_warnings {
            assert!(warning.contains("Risk:"));
            assert!(warning.contains("Consider setting"));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_check_token_mint_extensions_permanent_delegate() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let mint_with_delegate =
            create_mock_token2022_mint_with_extensions(6, vec![ExtensionType::PermanentDelegate]);
        let mint_pubkey = Pubkey::new_unique();

        let rpc_client = create_mock_rpc_client_with_account(&mint_with_delegate);
        let mut warnings = Vec::new();

        ConfigValidator::check_token_mint_extensions(
            &rpc_client,
            &[mint_pubkey.to_string()],
            &mut warnings,
        )
        .await;

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("PermanentDelegate extension"));
        assert!(warnings[0].contains(&mint_pubkey.to_string()));
        assert!(warnings[0].contains("Risk:"));
        assert!(warnings[0].contains("permanent delegate can transfer or burn tokens"));
    }

    #[tokio::test]
    #[serial]
    async fn test_check_token_mint_extensions_transfer_hook() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let mint_with_hook =
            create_mock_token2022_mint_with_extensions(6, vec![ExtensionType::TransferHook]);
        let mint_pubkey = Pubkey::new_unique();

        let rpc_client = create_mock_rpc_client_with_account(&mint_with_hook);
        let mut warnings = Vec::new();

        ConfigValidator::check_token_mint_extensions(
            &rpc_client,
            &[mint_pubkey.to_string()],
            &mut warnings,
        )
        .await;

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("TransferHook extension"));
        assert!(warnings[0].contains(&mint_pubkey.to_string()));
        assert!(warnings[0].contains("Risk:"));
        assert!(warnings[0].contains("custom program executes on every transfer"));
    }

    #[tokio::test]
    #[serial]
    async fn test_check_token_mint_extensions_both() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let mint_with_both = create_mock_token2022_mint_with_extensions(
            6,
            vec![ExtensionType::PermanentDelegate, ExtensionType::TransferHook],
        );
        let mint_pubkey = Pubkey::new_unique();

        let rpc_client = create_mock_rpc_client_with_account(&mint_with_both);
        let mut warnings = Vec::new();

        ConfigValidator::check_token_mint_extensions(
            &rpc_client,
            &[mint_pubkey.to_string()],
            &mut warnings,
        )
        .await;

        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("PermanentDelegate extension")));
        assert!(warnings.iter().any(|w| w.contains("TransferHook extension")));
    }

    #[tokio::test]
    #[serial]
    async fn test_check_token_mint_extensions_no_risky_extensions() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let mint_with_safe =
            create_mock_token2022_mint_with_extensions(6, vec![ExtensionType::MintCloseAuthority]);
        let mint_pubkey = Pubkey::new_unique();

        let rpc_client = create_mock_rpc_client_with_account(&mint_with_safe);
        let mut warnings = Vec::new();

        ConfigValidator::check_token_mint_extensions(
            &rpc_client,
            &[mint_pubkey.to_string()],
            &mut warnings,
        )
        .await;

        assert_eq!(warnings.len(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transactions_warning_when_enabled() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: true,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(warnings.iter().any(|w| w.contains("allow_durable_transactions is enabled")));
        assert!(warnings.iter().any(|w| w.contains("hold signed transactions indefinitely")));
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transactions_no_warning_when_disabled() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(!warnings.iter().any(|w| w.contains("allow_durable_transactions")));
    }

    #[tokio::test]
    #[serial]
    async fn test_jupiter_price_source_requires_api_key() {
        std::env::remove_var("JUPITER_API_KEY");

        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("JUPITER_API_KEY")));
        assert!(errors.iter().any(|e| e.contains("price_source = Jupiter")));
    }

    #[tokio::test]
    #[serial]
    async fn test_lighthouse_enabled_not_in_allowed_programs_error() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                lighthouse: LighthouseConfig {
                    enabled: true,
                    fail_if_transaction_size_overflow: true,
                },
                ..Default::default()
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.contains("Lighthouse is enabled but")
                && e.contains("is not in allowed_programs")));
    }

    #[tokio::test]
    #[serial]
    async fn test_lighthouse_enabled_in_disallowed_accounts_error() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                    LIGHTHOUSE_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![LIGHTHOUSE_PROGRAM_ID.to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                lighthouse: LighthouseConfig {
                    enabled: true,
                    fail_if_transaction_size_overflow: true,
                },
                ..Default::default()
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.contains("Lighthouse is enabled but")
                && e.contains("is in disallowed_accounts")));
    }

    #[tokio::test]
    #[serial]
    async fn test_lighthouse_disabled_no_validation() {
        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig {
                lighthouse: LighthouseConfig {
                    enabled: false,
                    fail_if_transaction_size_overflow: true,
                },
                ..Default::default()
            },
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();

        assert!(!warnings.iter().any(|w| w.contains("Lighthouse")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_sign_timeout_zero() {
        let config = crate::tests::config_mock::ConfigMockBuilder::new().build();
        let _ = crate::state::update_config(config);
        let mut config = crate::state::get_config().unwrap().clone();
        config.kora.sign_timeout_seconds = 0;
        crate::state::update_config(config.clone()).unwrap();

        let rpc_client = crate::tests::rpc_mock::RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;

        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert!(errors
            .iter()
            .any(|e| e.contains("sign_timeout_seconds must be at least 1 second")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_rejects_zero_fee_payer_balance_interval_when_enabled() {
        let mut config = crate::tests::config_mock::ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.metrics.enabled = true;
        config.metrics.fee_payer_balance.enabled = true;
        config.metrics.fee_payer_balance.expiry_seconds = 0;
        crate::state::update_config(config).unwrap();

        let rpc_client = crate::tests::rpc_mock::RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;

        assert!(result.is_err());
        let errors = result.err().unwrap();
        assert!(errors
            .iter()
            .any(|e| e
                .contains("metrics.fee_payer_balance.expiry_seconds must be at least 1 second")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_allows_zero_fee_payer_balance_interval_when_disabled() {
        let mut config = crate::tests::config_mock::ConfigMockBuilder::new().build();
        config.kora.cache.enabled = false;
        config.metrics.enabled = true;
        config.metrics.fee_payer_balance.enabled = false;
        config.metrics.fee_payer_balance.expiry_seconds = 0;
        crate::state::update_config(config).unwrap();

        let rpc_client = crate::tests::rpc_mock::RpcMockBuilder::new().build();
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_unvalidated_programs_no_warnings_for_standard_programs() {
        let allowed = vec![
            SYSTEM_PROGRAM_ID.to_string(),
            SPL_TOKEN_PROGRAM_ID.to_string(),
            TOKEN_2022_PROGRAM_ID.to_string(),
            solana_address_lookup_table_interface::program::ID.to_string(),
            spl_associated_token_account_interface::program::id().to_string(),
            solana_compute_budget_interface::id().to_string(),
            LIGHTHOUSE_PROGRAM_ID.to_string(),
        ];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert!(warnings.is_empty(), "Expected no warnings for known programs, got: {warnings:?}");
    }

    #[test]
    fn test_warn_mutable_transfer_hook_payment_risk() {
        use crate::{fee::price::PriceModel, tests::config_mock::ConfigMockBuilder};

        let build = |policy: TransferHookPolicy, model: PriceModel| {
            let mut config = ConfigMockBuilder::new().build();
            config.validation.token_2022.transfer_hook_policy = policy;
            config.validation.price.model = model;
            config
        };

        let config = build(
            TransferHookPolicy::DenyMutableForDelayedSigning,
            PriceModel::Margin { margin: 0.0 },
        );
        let mut warnings = Vec::new();
        ConfigValidator::warn_mutable_transfer_hook_payment_risk(&config.validation, &mut warnings);
        assert!(warnings.iter().any(|w| w.contains("mutable Token-2022 transfer hooks")));

        let config = build(TransferHookPolicy::DenyAll, PriceModel::Margin { margin: 0.0 });
        let mut warnings = Vec::new();
        ConfigValidator::warn_mutable_transfer_hook_payment_risk(&config.validation, &mut warnings);
        assert!(warnings.is_empty());

        let config = build(TransferHookPolicy::AllowAll, PriceModel::Free);
        let mut warnings = Vec::new();
        ConfigValidator::warn_mutable_transfer_hook_payment_risk(&config.validation, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_warn_unvalidated_programs_warns_for_vote_program() {
        use crate::constant::VOTE_PROGRAM_ID;
        let allowed = vec![SYSTEM_PROGRAM_ID.to_string(), VOTE_PROGRAM_ID.to_string()];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Vote Program"));
        assert!(warnings[0].contains(&VOTE_PROGRAM_ID.to_string()));
    }

    #[test]
    fn test_warn_unvalidated_programs_warns_for_stake_program() {
        use crate::constant::STAKE_PROGRAM_ID;
        let allowed = vec![SYSTEM_PROGRAM_ID.to_string(), STAKE_PROGRAM_ID.to_string()];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Stake Program"));
    }

    #[test]
    fn test_warn_unvalidated_programs_no_warning_for_bpf_loader_upgradeable() {
        // Has a dedicated fee-payer parser and is in `known_programs`; no warning expected.
        let allowed =
            vec![SYSTEM_PROGRAM_ID.to_string(), BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string()];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert!(
            warnings.is_empty(),
            "BPF Loader Upgradeable has a parser; no warning expected. got: {warnings:?}"
        );
    }

    #[test]
    fn test_warn_unvalidated_programs_warns_for_custom_program() {
        let custom = solana_sdk::pubkey::Pubkey::new_unique().to_string();
        let allowed = vec![SYSTEM_PROGRAM_ID.to_string(), custom.clone()];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(&custom));
        assert!(warnings[0].contains("no dedicated fee-payer instruction parser"));
    }

    #[test]
    fn test_warn_unvalidated_programs_no_double_warning_for_high_risk_programs() {
        use crate::constant::VOTE_PROGRAM_ID;
        let custom = solana_sdk::pubkey::Pubkey::new_unique().to_string();
        let allowed =
            vec![SYSTEM_PROGRAM_ID.to_string(), VOTE_PROGRAM_ID.to_string(), custom.clone()];
        let mut warnings = Vec::new();
        ConfigValidator::warn_unvalidated_programs(&allowed, &mut warnings);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("Vote Program")));
        assert!(warnings.iter().any(|w| w.contains(&custom)));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_signers_http_config() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        std::env::set_var("JUPITER_API_KEY", "test-api-key");
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_priority_fee_lamports: None,
                max_signatures: 10,
                allowed_programs: ProgramsConfig::Allowlist(vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ]),
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
                allow_durable_transactions: false,
                allowed_transaction_versions: default_allowed_transaction_versions(),
                max_price_staleness_slots: 0,
                require_one_of_programs: vec![],
                cross_cluster_check: false,
                cross_cluster_endpoints: vec![],
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        let _ = update_config(config);

        let toml_content = r#"
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "turnkey_signer_invalid"
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID"
public_key_env = "TURNKEY_PUBLIC_KEY"
http_config = { request_timeout_secs = 0, connect_timeout_secs = 0 }
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        std::env::set_var("TURNKEY_API_PUBLIC_KEY", "dummy");
        std::env::set_var("TURNKEY_API_PRIVATE_KEY", "dummy");
        std::env::set_var("TURNKEY_ORG_ID", "dummy");
        std::env::set_var("TURNKEY_PRIVATE_KEY_ID", "dummy");
        std::env::set_var("TURNKEY_PUBLIC_KEY", "7EcDhSYGxXyscszYEp35KHN8vvw3svAuLKTzXwCFLtV");

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );

        let result = ConfigValidator::validate_with_result_and_signers(
            &rpc_client,
            true,
            Some(temp_file.path()),
        )
        .await;

        std::env::remove_var("TURNKEY_API_PUBLIC_KEY");
        std::env::remove_var("TURNKEY_API_PRIVATE_KEY");
        std::env::remove_var("TURNKEY_ORG_ID");
        std::env::remove_var("TURNKEY_PRIVATE_KEY_ID");
        std::env::remove_var("TURNKEY_PUBLIC_KEY");
        std::env::remove_var("JUPITER_API_KEY");

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains(
            "request_timeout_secs must be greater than 0 for signer 'turnkey_signer_invalid'"
        )));
        assert!(errors.iter().any(|e| e.contains(
            "connect_timeout_secs must be greater than 0 for signer 'turnkey_signer_invalid'"
        )));
    }
}
