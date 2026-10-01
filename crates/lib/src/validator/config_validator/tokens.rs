use super::ConfigValidator;
use crate::{
    config::{Config, SplTokenConfig, Token2022Config, TransferHookPolicy, ValidationConfig},
    fee::price::PriceModel,
    token::token::TokenUtil,
};
use solana_sdk::pubkey::Pubkey;
use spl_token_2022_interface::{extension::ExtensionType, ID as TOKEN_2022_PROGRAM_ID};
use spl_token_interface::ID as SPL_TOKEN_PROGRAM_ID;
use std::str::FromStr;

impl ConfigValidator {
    pub(super) fn check_tokens(
        config: &Config,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        // Only required when fees are charged; Free-pricing operators need no allowlist.
        if config.validation.is_payment_required() && config.validation.allowed_tokens.is_empty() {
            errors.push("No allowed tokens configured".to_string());
        } else if !config.validation.allowed_tokens.is_empty() {
            if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.allowed_tokens) {
                errors.push(format!("Invalid token address: {e}"));
            }
        }

        if let Err(e) =
            TokenUtil::check_valid_tokens(config.validation.allowed_spl_paid_tokens.as_slice())
        {
            errors.push(format!("Invalid spl paid token address: {e}"));
        }

        if matches!(config.validation.allowed_spl_paid_tokens, SplTokenConfig::All) {
            warnings.push(
                "⚠️  Using 'All' for allowed_spl_paid_tokens - this accepts ANY SPL token for payment. \
                Consider using an explicit allowlist to reduce volatility risk and protect against \
                potentially malicious or worthless tokens being used for fees.".to_string()
            );
        }

        if let Err(e) = TokenUtil::check_valid_tokens(&config.validation.disallowed_accounts) {
            errors.push(format!("Invalid disallowed account address: {e}"));
        }

        if let Err(e) = validate_token2022_extensions(&config.validation.token_2022, warnings) {
            errors.push(format!("Token2022 extension validation failed: {e}"));
        }

        let has_token_program =
            config.validation.allowed_programs.contains(&SPL_TOKEN_PROGRAM_ID.to_string());
        let has_token22_program =
            config.validation.allowed_programs.contains(&TOKEN_2022_PROGRAM_ID.to_string());

        if has_token22_program
            && !config
                .validation
                .token_2022
                .is_mint_extension_blocked(ExtensionType::PermanentDelegate)
        {
            warnings.push(
                "⚠️  SECURITY: PermanentDelegate extension is NOT blocked. Tokens with this extension \
                allow the delegate to transfer/burn tokens at any time without owner approval. \
                This creates significant risks:\n\
                  - Payment tokens: Funds can be seized after payment\n\
                Consider adding \"permanent_delegate\" to blocked_mint_extensions in [validation.token_2022] \
                unless explicitly needed for your use case.".to_string()
            );
        }

        let fees_enabled = !matches!(config.validation.price.model, PriceModel::Free);

        if fees_enabled {
            if !has_token_program && !has_token22_program {
                errors.push("When fees are enabled, at least one token program (SPL Token or Token2022) must be in allowed_programs".to_string());
            }

            if !config.validation.allowed_spl_paid_tokens.has_tokens() {
                errors.push(
                    "When fees are enabled, allowed_spl_paid_tokens cannot be empty".to_string(),
                );
            }
        } else {
            warnings.push(
                "⚠️  SECURITY: Free pricing model enabled - all transactions will be processed \
                without charging fees."
                    .to_string(),
            );
        }

        for paid_token in &config.validation.allowed_spl_paid_tokens {
            if !config.validation.allowed_tokens.contains(paid_token) {
                errors.push(format!(
                    "Token {paid_token} in allowed_spl_paid_tokens must also be in allowed_tokens"
                ));
            }
        }
    }

    pub(super) fn warn_mutable_transfer_hook_payment_risk(
        validation: &ValidationConfig,
        warnings: &mut Vec<String>,
    ) {
        let allows_immediate_mutable_hook =
            !matches!(validation.token_2022.transfer_hook_policy, TransferHookPolicy::DenyAll);
        if allows_immediate_mutable_hook && validation.is_payment_required() {
            warnings.push(
                "⚠️  SECURITY: transfer_hook_policy allows mutable Token-2022 transfer hooks on \
                immediate signAndSend. A payment mint with a mutable transfer hook can refund \
                Kora's payment after validation but before execution, leaving zero net payment. \
                Use transfer_hook_policy = \"deny_all\", or only accept payment mints with \
                immutable transfer hooks."
                    .to_string(),
            );
        }
    }

    pub(super) fn check_price_model(
        config: &Config,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        match &config.validation.price.model {
            PriceModel::Fixed { amount, token, strict } => {
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

                let has_auth = config.kora.auth.has_resolved_auth();
                if !has_auth {
                    warnings.push(
                        "⚠️  SECURITY: Fixed pricing with NO authentication enabled. \
                        Without authentication, anyone can spam transactions at your expense. \
                        Consider enabling api_keys or hmac_secret in [kora.auth]."
                            .to_string(),
                    );
                }

                if *strict {
                    warnings.push(
                        "Strict pricing mode enabled. \
                        Transactions where fee payer outflow exceeds the fixed price will be rejected."
                            .to_string(),
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
    }
}

/// Validate Token2022 extension configuration
pub(super) fn validate_token2022_extensions(
    config: &Token2022Config,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    config.clone().initialize()?;

    match config.transfer_hook_policy {
        TransferHookPolicy::AllowAll => {
            warnings.push(
                "⚠️  SECURITY: transfer_hook_policy is 'allow_all'. \
                Mutable transfer-hook authorities are permitted on ALL signing flows. \
                Risk: A malicious hook authority can swap the hook program to drain your fee payer. \
                Consider setting transfer_hook_policy = \"deny_all\"."
                    .to_string(),
            );
        }
        TransferHookPolicy::DenyMutableForDelayedSigning => {
            warnings.push(
                "⚠️  SECURITY: transfer_hook_policy is 'deny_mutable_for_delayed_signing'. \
                Mutable transfer-hook authorities are permitted on signAndSendTransaction / signAndSendBundle. \
                Risk: A hook authority can swap the hook program between signing and execution. \
                Consider setting transfer_hook_policy = \"deny_all\" for maximum protection."
                    .to_string(),
            );
        }
        TransferHookPolicy::DenyAll => {}
    }

    Ok(())
}
