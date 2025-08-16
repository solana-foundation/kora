use std::str::FromStr;

use crate::{
    admin::token_util::find_missing_atas,
    fee::price::PriceModel,
    oracle::PriceSource,
    state::get_config,
    token::token::TokenUtil,
    validator::account_validator::{validate_account, AccountType},
    KoraError,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, system_program::ID as SYSTEM_PROGRAM_ID};
use spl_token::ID as SPL_TOKEN_PROGRAM_ID;
use spl_token_2022::ID as TOKEN_2022_PROGRAM_ID;

#[cfg(test)]
use crate::config::{FeePayerPolicy, ValidationConfig};
#[cfg(test)]
use crate::fee::price::PriceConfig;

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
        if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.allowed_spl_paid_tokens) {
            errors.push(format!("Invalid spl paid token address: {e}"));
        }

        // Validate disallowed accounts
        if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.disallowed_accounts) {
            errors.push(format!("Invalid disallowed account address: {e}"));
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
            if config.validation.allowed_spl_paid_tokens.is_empty() {
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
                if !config.validation.allowed_spl_paid_tokens.contains(token) {
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
                let payment_address = Pubkey::from_str(payment_address).unwrap();

                let atas_to_create = find_missing_atas(rpc_client, &payment_address).await;

                if let Err(e) = atas_to_create {
                    errors.push(format!("Failed to find missing ATAs: {e}"));
                } else if !atas_to_create.unwrap().is_empty() {
                    errors.push(format!("Missing ATAs for payment address: {payment_address}"));
                }
            }
        }

        // Output results
        println!("=== Configuration Validation ===");
        if errors.is_empty() {
            println!("✓ Configuration validation successful!");
            println!("\n=== Current Configuration ===");
            println!("{config:#?}");
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

#[cfg(test)]
mod tests {

    use crate::{
        config::{AuthConfig, Config, EnabledMethods, KoraConfig, MetricsConfig},
        fee::price::PriceConfig,
        state::update_config,
        tests::common::{
            create_mock_non_executable_account, create_mock_program_account,
            create_mock_rpc_client_account_not_found, create_mock_spl_mint_account,
            get_mock_rpc_client,
        },
    };
    use serial_test::serial;

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
                allowed_spl_paid_tokens: vec!["token3".to_string()],
                disallowed_accounts: vec!["account1".to_string()],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config.clone());

        // Test empty tokens list
        config.validation.allowed_tokens = vec![];
        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig::default(),
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = ConfigValidator::validate_with_result(&rpc_client, true).await;
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(warnings.is_empty());
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
                auth: AuthConfig::default(),
                payment_address: None,
            },
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
                allowed_spl_paid_tokens: vec!["invalid_token_address".to_string()], // Error - invalid token
                disallowed_accounts: vec!["invalid_account_address".to_string()], // Error - invalid account
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig {
                    model: PriceModel::Margin { margin: -0.1 }, // Error - negative margin
                },
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
                allowed_spl_paid_tokens: vec![], // Empty when fees enabled - should error
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Margin { margin: 0.1 } },
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        let _ = update_config(config);

        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
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
                allowed_spl_paid_tokens: vec![
                    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string()
                ],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
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
                allowed_spl_paid_tokens: vec![], // Empty to avoid duplicate validation
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
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

        let mock_account = create_mock_program_account();
        let rpc_client = get_mock_rpc_client(&mock_account);

        // Test with RPC validation enabled (skip_rpc_validation = false)
        // The program validation should pass, but token validation will fail (AccountNotFound)
        let result = ConfigValidator::validate_with_result(&rpc_client, false).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have token validation errors (account not found), but no program validation errors
        assert!(errors.iter().any(|e| e.contains("Token")
            && e.contains("validation failed")
            && e.contains("AccountNotFound")));
        assert!(!errors.iter().any(|e| e.contains("Program") && e.contains("validation failed")));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_with_result_rpc_validation_valid_token_mint() {
        let config = create_token_only_config();

        // Initialize global config
        let _ = update_config(config);

        let mock_account = create_mock_spl_mint_account(6);
        let rpc_client = get_mock_rpc_client(&mock_account);

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
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
            },
            metrics: MetricsConfig::default(),
            kora: KoraConfig::default(),
        };

        // Initialize global config
        let _ = update_config(config);

        let mock_account = create_mock_non_executable_account(); // Non-executable
        let rpc_client = get_mock_rpc_client(&mock_account);

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
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
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
        assert!(errors.iter().any(|e| e.contains("Failed to get account")));
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
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Jupiter,
                fee_payer_policy: FeePayerPolicy::default(),
                price: PriceConfig { model: PriceModel::Free },
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
}
