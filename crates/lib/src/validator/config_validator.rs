use std::{path::Path, str::FromStr};

use crate::{
    admin::token_util::find_missing_atas,
    config::{SplTokenConfig, Token2022Config},
    fee::price::PriceModel,
    oracle::PriceSource,
    signer::SignerPoolConfig,
    state::get_config,
    token::{spl_token_2022_util, token::TokenUtil},
    validator::{
        account_validator::{validate_account, AccountType},
        cache_validator::CacheValidator,
        signer_validator::SignerValidator,
    },
    KoraError,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, system_program::ID as SYSTEM_PROGRAM_ID};
use spl_token::ID as SPL_TOKEN_PROGRAM_ID;
use spl_token_2022::{extension::ExtensionType, ID as TOKEN_2022_PROGRAM_ID};

pub struct ConfigValidator {}

impl ConfigValidator {
    pub async fn validate(_rpc_client: &RpcClient) -> Result<(), KoraError> {
        let config = &get_config()?;

        if config.validation.allowed_tokens.is_empty() {
            return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
        }

        TokenUtil::check_valid_tokens(&config.validation.allowed_tokens)?;

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

        // Validate rate limit (warn if 0)
        if config.kora.rate_limit == 0 {
            warnings.push("Rate limit is set to 0 - this will block all requests".to_string());
        }

        // Validate payment address
        if let Some(payment_address) = &config.kora.payment_address {
            if let Err(e) = Pubkey::from_str(payment_address) {
                errors.push(format!("Invalid payment address: {e}"));
            }
        }

        // Validate enabled methods (warn if all false)
        let methods = &config.kora.enabled_methods;
        if !methods.iter().any(|enabled| enabled) {
            warnings.push(
                "All rpc methods are disabled - this will block all functionality".to_string(),
            );
        }

        // Validate max allowed lamports (warn if 0)
        if config.validation.max_allowed_lamports == 0 {
            warnings
                .push("Max allowed lamports is 0 - this will block all SOL transfers".to_string());
        }

        // Validate max signatures (warn if 0)
        if config.validation.max_signatures == 0 {
            warnings.push("Max signatures is 0 - this will block all transactions".to_string());
        }

        // Validate price source (warn if Mock)
        if matches!(config.validation.price_source, PriceSource::Mock) {
            warnings.push("Using Mock price source - not suitable for production".to_string());
        }

        // Validate allowed programs (warn if empty or missing system/token programs)
        if config.validation.allowed_programs.is_empty() {
            warnings.push(
                "No allowed programs configured - this will block all transactions".to_string(),
            );
        } else {
            if !config.validation.allowed_programs.contains(&SYSTEM_PROGRAM_ID.to_string()) {
                warnings.push("Missing System Program in allowed programs - SOL transfers and account operations will be blocked".to_string());
            }
            if !config.validation.allowed_programs.contains(&SPL_TOKEN_PROGRAM_ID.to_string())
                && !config.validation.allowed_programs.contains(&TOKEN_2022_PROGRAM_ID.to_string())
            {
                warnings.push("Missing Token Program in allowed programs - SPL token operations will be blocked".to_string());
            }
        }

        // Validate allowed tokens
        if config.validation.allowed_tokens.is_empty() {
            errors.push("No allowed tokens configured".to_string());
        } else if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.allowed_tokens) {
            errors.push(format!("Invalid token address: {e}"));
        }

        // Validate allowed spl paid tokens
        if let Err(e) =
            TokenUtil::check_valid_tokens(config.validation.allowed_spl_paid_tokens.as_slice())
        {
            errors.push(format!("Invalid spl paid token address: {e}"));
        }

        // Warn if using "All" for allowed_spl_paid_tokens
        if matches!(config.validation.allowed_spl_paid_tokens, SplTokenConfig::All) {
            warnings.push(
                "⚠️  Using 'All' for allowed_spl_paid_tokens - this accepts ANY SPL token for payment. \
                Consider using an explicit allowlist to reduce volatility risk and protect against \
                potentially malicious or worthless tokens being used for fees.".to_string()
            );
        }

        // Validate disallowed accounts
        if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.disallowed_accounts) {
            errors.push(format!("Invalid disallowed account address: {e}"));
        }

        // Validate Token2022 extensions
        if let Err(e) = validate_token2022_extensions(&config.validation.token_2022) {
            errors.push(format!("Token2022 extension validation failed: {e}"));
        }

        // Warn if PermanentDelegate is not blocked
        if !config.validation.token_2022.is_mint_extension_blocked(ExtensionType::PermanentDelegate)
        {
            warnings.push(
                "⚠️  SECURITY: PermanentDelegate extension is NOT blocked. Tokens with this extension \
                allow the delegate to transfer/burn tokens at any time without owner approval. \
                This creates significant risks:\n\
                  - Payment tokens: Funds can be seized after payment\n\
                Consider adding \"permanent_delegate\" to blocked_mint_extensions in [validation.token2022] \
                unless explicitly needed for your use case.".to_string()
            );
        }

        // Check if fees are enabled (not Free pricing)
        let fees_enabled = !matches!(config.validation.price.model, PriceModel::Free);

        if fees_enabled {
            // If fees enabled, token or token22 must be enabled in allowed_programs
            let has_token_program =
                config.validation.allowed_programs.contains(&SPL_TOKEN_PROGRAM_ID.to_string());
            let has_token22_program =
                config.validation.allowed_programs.contains(&TOKEN_2022_PROGRAM_ID.to_string());

            if !has_token_program && !has_token22_program {
                errors.push("When fees are enabled, at least one token program (SPL Token or Token2022) must be in allowed_programs".to_string());
            }

            // If fees enabled, allowed_spl_paid_tokens can't be empty
            if !config.validation.allowed_spl_paid_tokens.has_tokens() {
                errors.push(
                    "When fees are enabled, allowed_spl_paid_tokens cannot be empty".to_string(),
                );
            }
        }

        // Validate that all tokens in allowed_spl_paid_tokens are also in allowed_tokens
        for paid_token in &config.validation.allowed_spl_paid_tokens {
            if !config.validation.allowed_tokens.contains(paid_token) {
                errors.push(format!(
                    "Token {paid_token} in allowed_spl_paid_tokens must also be in allowed_tokens"
                ));
            }
        }

        // Validate margin (error if negative)
        match &config.validation.price.model {
            PriceModel::Fixed { amount, token } => {
                if *amount == 0 {
                    warnings
                        .push("Fixed price amount is 0 - transactions will be free".to_string());
                }
                if Pubkey::from_str(token).is_err() {
                    errors.push(format!("Invalid token address for fixed price: {token}"));
                }
                if !config.validation.supports_token(token) {
                    errors.push(format!(
                        "Token address for fixed price is not in allowed spl paid tokens: {token}"
                    ));
                }

                // Warn about dangerous configurations with fixed pricing
                let has_auth =
                    config.kora.auth.api_key.is_some() || config.kora.auth.hmac_secret.is_some();
                if !has_auth {
                    warnings.push(
                        "⚠️  SECURITY: Fixed pricing with NO authentication enabled. \
                        Without authentication, anyone can spam transactions at your expense. \
                        Consider enabling api_key or hmac_secret in [kora.auth]."
                            .to_string(),
                    );
                }

                if config.validation.fee_payer_policy.system.allow_transfer {
                    warnings.push(
                        "⚠️  SECURITY: Fixed pricing with allow_transfer=true for System instructions. \
                        Users can make the fee payer transfer arbitrary SOL amounts at fixed cost. \
                        This can drain your fee payer account. \
                        Consider setting [validation.fee_payer_policy.system] allow_transfer=false.".to_string()
                    );
                }

                if config.validation.fee_payer_policy.spl_token.allow_transfer {
                    warnings.push(
                        "⚠️  SECURITY: Fixed pricing with allow_transfer=true for SPL Token instructions. \
                        Users can make the fee payer transfer arbitrary token amounts at fixed cost. \
                        This can drain your fee payer token accounts. \
                        Consider setting [validation.fee_payer_policy.spl_token] allow_transfer=false.".to_string()
                    );
                }

                if config.validation.fee_payer_policy.token_2022.allow_transfer {
                    warnings.push(
                        "⚠️  SECURITY: Fixed pricing with allow_transfer=true for Token2022 instructions. \
                        Users can make the fee payer transfer arbitrary token amounts at fixed cost. \
                        This can drain your fee payer token accounts. \
                        Consider setting [validation.fee_payer_policy.token_2022] allow_transfer=false.".to_string()
                    );
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

        // General authentication warning
        let has_auth = config.kora.auth.api_key.is_some() || config.kora.auth.hmac_secret.is_some();
        if !has_auth {
            warnings.push(
                "⚠️  SECURITY: No authentication configured (neither api_key nor hmac_secret). \
                Authentication is strongly recommended for production deployments. \
                Consider enabling api_key or hmac_secret in [kora.auth]."
                    .to_string(),
            );
        }

        // Validate usage limit configuration
        let usage_config = &config.kora.usage_limit;
        if usage_config.enabled {
            let (usage_errors, usage_warnings) = CacheValidator::validate(usage_config).await;
            errors.extend(usage_errors);
            warnings.extend(usage_warnings);
        }

        // RPC validation - only if not skipped
        if !skip_rpc_validation {
            // Validate allowed programs - should be executable
            for program_str in &config.validation.allowed_programs {
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
            for token_str in &config.validation.allowed_tokens {
                if let Ok(token_pubkey) = Pubkey::from_str(token_str) {
                    if let Err(e) =
                        validate_account(rpc_client, &token_pubkey, Some(AccountType::Mint)).await
                    {
                        errors.push(format!("Token {token_str} validation failed: {e}"));
                    }
                }
            }

            // Validate allowed spl paid tokens - should be non-executable token mints
            for token_str in &config.validation.allowed_spl_paid_tokens {
                if let Ok(token_pubkey) = Pubkey::from_str(token_str) {
                    if let Err(e) =
                        validate_account(rpc_client, &token_pubkey, Some(AccountType::Mint)).await
                    {
                        errors.push(format!("SPL paid token {token_str} validation failed: {e}"));
                    }
                }
            }

            // Validate missing ATAs for payment address
            if let Some(payment_address) = &config.kora.payment_address {
                if let Ok(payment_address) = Pubkey::from_str(payment_address) {
                    match find_missing_atas(rpc_client, &payment_address).await {
                        Ok(atas_to_create) => {
                            if !atas_to_create.is_empty() {
                                errors.push(format!(
                                    "Missing ATAs for payment address: {payment_address}"
                                ));
                            }
                        }
                        Err(e) => errors.push(format!("Failed to find missing ATAs: {e}")),
                    }
                } else {
                    errors.push(format!("Invalid payment address: {payment_address}"));
                }
            }
        }

        // Validate signers configuration if provided
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

        // Output results
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

/// Validate Token2022 extension configuration
fn validate_token2022_extensions(config: &Token2022Config) -> Result<(), String> {
    // Validate blocked mint extensions
    for ext_name in &config.blocked_mint_extensions {
        if spl_token_2022_util::parse_mint_extension_string(ext_name).is_none() {
            return Err(format!(
                "Invalid mint extension name: '{ext_name}'. Valid names are: {:?}",
                spl_token_2022_util::get_all_mint_extension_names()
            ));
        }
    }

    // Validate blocked account extensions
    for ext_name in &config.blocked_account_extensions {
        if spl_token_2022_util::parse_account_extension_string(ext_name).is_none() {
            return Err(format!(
                "Invalid account extension name: '{ext_name}'. Valid names are: {:?}",
                spl_token_2022_util::get_all_account_extension_names()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{
            AuthConfig, CacheConfig, Config, EnabledMethods, FeePayerPolicy, KoraConfig,
            MetricsConfig, SplTokenConfig, UsageLimitConfig, ValidationConfig,
        },
        constant::DEFAULT_MAX_REQUEST_BODY_SIZE,
        fee::price::PriceConfig,
        state::update_config,
        tests::common::{
            create_mock_non_executable_account, create_mock_program_account,
            create_mock_rpc_client_account_not_found, create_mock_rpc_client_with_account,
            create_mock_rpc_client_with_mint, RpcMockBuilder,
        },
    };
    use serial_test::serial;
    use solana_commitment_config::CommitmentConfig;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_validate_config() {
        let mut config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1000000000,
                max_signatures: 10,
                allowed_programs: vec!["program1".to_string()],
                allowed_tokens: vec!["token1".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec!["token3".to_string()]),
                disallowed_accounts: vec!["account1".to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config.clone());

        // Test empty tokens list
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
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: Token2022Config::default(),
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        // Expect warnings about PermanentDelegate and no authentication
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().any(|w| w.contains("PermanentDelegate")));
        assert!(warnings.iter().any(|w| w.contains("No authentication configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_warnings() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 0,  // Should warn
                max_signatures: 0,        // Should warn
                allowed_programs: vec![], // Should warn
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock, // Should warn
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            kora: KoraConfig {
                rate_limit: 0, // Should warn
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
                },
                auth: AuthConfig::default(),
                payment_address: None,
                cache: CacheConfig::default(),
                usage_limit: UsageLimitConfig::default(),
            },
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = RpcClient::new_with_commitment(
            "http://localhost:8899".to_string(),
            CommitmentConfig::confirmed(),
        );
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
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
    #[serial]
    async fn test_validate_with_result_missing_system_program_warning() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec!["SomeOtherProgram".to_string()], // Missing system program
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
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
    async fn test_validate_with_result_errors() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec![], // Error - no tokens
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "invalid_token_address".to_string()
                ]), // Error - invalid token
                disallowed_accounts: vec!["invalid_account_address".to_string()], // Error - invalid account
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Margin { margin: -0.1 }, // Error - negative margin
                },
                token_2022: Token2022Config::default(),
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0,                                  // Should warn
                        token: "invalid_token_address".to_string(), // Should error
                    },
                },
                token_2022: Token2022Config::default(),
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
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
                        token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // Valid but not in allowed
                    },
                },
                token_2022: Token2022Config::default(),
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
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Fixed {
                        amount: 0, // Should warn
                        token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                    },
                },
                token_2022: Token2022Config::default(),
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()], // Missing token programs
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]), // Empty when fees enabled - should error
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
                token_2022: Token2022Config::default(),
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
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::All, // All tokens are allowed
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcMockBuilder::new().build();

        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());

        // Check that it warns about using "All" for allowed_spl_paid_tokens
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
                max_signatures: 10,
                allowed_programs: vec![
                    SYSTEM_PROGRAM_ID.to_string(),
                    SPL_TOKEN_PROGRAM_ID.to_string(),
                ],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // Not in allowed_tokens
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
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

    // Helper to create a simple test that only validates programs (no tokens)
    fn create_program_only_config() -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()], // Required to pass basic validation
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
                ]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
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
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]), // Empty to avoid duplicate validation
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_valid_program() {
        let config = create_program_only_config();

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_account(&create_mock_program_account());

        // Test with RPC validation enabled (skip_rpc_validation = false)
        // The program validation should pass, but token validation will fail (AccountNotFound)
        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have token validation errors (account not found), but no program validation errors
        assert!(errors.iter().any(|e| e.contains("Token")
            && e.contains("validation failed")
            && e.contains("not found")));
        assert!(!errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_valid_token_mint() {
        let config = create_token_only_config();

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_mint(6);

        // Test with RPC validation enabled (skip_rpc_validation = false)
        // Token validation should pass (mock returns token mint) since we have no programs
        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_ok());
        // Should have warnings about no programs but no errors
        let warnings = result.unwrap();
        assert!(warnings.iter().any(|w| w.contains("No allowed programs configured")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_non_executable_program_fails() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_with_account(&create_mock_non_executable_account());

        // Test with RPC validation enabled (skip_rpc_validation = false)
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = create_mock_rpc_client_account_not_found();

        // Test with RPC validation enabled (skip_rpc_validation = false)
        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 2, "Should have validation errors for programs and tokens");
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_skip_rpc_validation() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
                allowed_tokens: vec!["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()],
                allowed_spl_paid_tokens: SplTokenConfig::Allowlist(vec![]),
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
                token_2022: Token2022Config::default(),
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        // Use account not found RPC client - should not matter when skipping RPC validation
        let rpc_client = create_mock_rpc_client_account_not_found();

        // Test with RPC validation disabled (skip_rpc_validation = true)
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok()); // Should pass because RPC validation is skipped
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_valid_token2022_extensions() {
        let config = Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1_000_000,
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
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
                max_signatures: 10,
                allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
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

        let result = validate_token2022_extensions(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_token2022_extensions_invalid_mint_extension() {
        let mut config = Token2022Config::default();
        config.blocked_mint_extensions = vec!["invalid_extension".to_string()];

        let result = validate_token2022_extensions(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid mint extension name: 'invalid_extension'"));
    }

    #[test]
    fn test_validate_token2022_extensions_invalid_account_extension() {
        let mut config = Token2022Config::default();
        config.blocked_account_extensions = vec!["invalid_extension".to_string()];

        let result = validate_token2022_extensions(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid account extension name: 'invalid_extension'"));
    }

    #[test]
    fn test_validate_token2022_extensions_empty() {
        let config = Token2022Config::default();

        let result = validate_token2022_extensions(&config);
        assert!(result.is_ok());
    }
}
