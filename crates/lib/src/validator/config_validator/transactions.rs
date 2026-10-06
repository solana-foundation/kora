use super::ConfigValidator;
use crate::{
    config::Config,
    constant::{
        BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LIGHTHOUSE_PROGRAM_ID, LOADER_V4_PROGRAM_ID,
        STAKE_PROGRAM_ID, VOTE_PROGRAM_ID,
    },
    oracle::PriceSource,
};
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
use solana_transaction::versioned::TransactionVersion;
use spl_token_2022_interface::ID as TOKEN_2022_PROGRAM_ID;
use spl_token_interface::ID as SPL_TOKEN_PROGRAM_ID;
use std::{collections::HashSet, str::FromStr};

impl ConfigValidator {
    pub(super) fn check_transaction_settings(
        config: &Config,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        if config.validation.max_allowed_lamports == 0 {
            warnings
                .push("Max allowed lamports is 0 - this will block all SOL transfers".to_string());
        }

        if config.validation.max_signatures == 0 {
            warnings.push("Max signatures is 0 - this will block all transactions".to_string());
        }

        if config.validation.allowed_transaction_versions.is_empty() {
            errors.push(
                "allowed_transaction_versions is empty - this will block all transactions. \
                Remove it to allow all versions, or list the versions to accept"
                    .to_string(),
            );
        }

        for version in &config.validation.allowed_transaction_versions {
            if let TransactionVersion::Number(number) = version {
                if *number > 1 {
                    errors.push(format!(
                        "allowed_transaction_versions contains unsupported version {number}. \
                        Supported versions are \"legacy\", 0 and 1"
                    ));
                }
            }
        }

        if matches!(config.validation.price_source, PriceSource::Mock) {
            warnings.push("Using Mock price source - not suitable for production".to_string());
        }

        if matches!(config.validation.price_source, PriceSource::Jupiter)
            && std::env::var("JUPITER_API_KEY").is_err()
        {
            errors.push(
                "JUPITER_API_KEY environment variable not set. Required when price_source = Jupiter".to_string()
            );
        }

        if config.validation.allow_durable_transactions {
            warnings.push(
                "⚠️  SECURITY: allow_durable_transactions is enabled. \
                Risk: Users can hold signed transactions indefinitely and execute them much later. \
                Token values may change, your fee payer may run low on funds, or you may no longer \
                want to subsidize these transactions. \
                Consider disabling durable transactions unless specifically required."
                    .to_string(),
            );
        }
    }

    pub(super) fn check_programs(
        config: &Config,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        let is_wildcard = config.validation.allowed_programs.is_all();
        if is_wildcard {
            warnings.push(
                "allowed_programs is set to \"All\" — WILDCARD MODE: any program will be \
                 accepted. Ensure upstream policy enforcement and fee_payer_policy / \
                 disallowed_accounts are configured to bound drainage risk."
                    .to_string(),
            );
        } else if config.validation.allowed_programs.as_slice().is_empty() {
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

        if !is_wildcard {
            Self::warn_unvalidated_programs(
                config.validation.allowed_programs.as_slice(),
                warnings,
            );
        }

        if config.kora.lighthouse.enabled {
            let lighthouse_program = LIGHTHOUSE_PROGRAM_ID.to_string();
            if !is_wildcard && !config.validation.allowed_programs.contains(&lighthouse_program) {
                errors.push(format!(
                    "Lighthouse is enabled but {} is not in allowed_programs. Consider adding it to allowed_programs.",
                    LIGHTHOUSE_PROGRAM_ID
                ));
            }
            if config.validation.disallowed_accounts.contains(&lighthouse_program) {
                errors.push(format!(
                    "Lighthouse is enabled but {} is in disallowed_accounts. Consider removing it from disallowed_accounts.",
                    LIGHTHOUSE_PROGRAM_ID
                ));
            }

            let enabled_methods = &config.kora.enabled_methods;
            let mut unprotected_methods = Vec::new();
            if enabled_methods.sign_and_send_transaction {
                unprotected_methods.push("signAndSendTransaction");
            }
            if enabled_methods.sign_and_send_bundle {
                unprotected_methods.push("signAndSendBundle");
            }
            if !unprotected_methods.is_empty() {
                warnings.push(format!(
                    "Lighthouse is enabled but {} will NOT have fee payer protection. \
                    These methods send transactions directly to the network, so adding assertions \
                    would invalidate client signatures. Consider using signTransaction/signBundle instead.",
                    unprotected_methods.join(", ")
                ));
            }
        }

        let create_account_only_via =
            &config.validation.fee_payer_policy.system.create_account_only_via;
        for (field, pubkeys) in [
            ("allowed_programs", config.validation.allowed_programs.as_slice()),
            ("require_one_of_programs", config.validation.require_one_of_programs.as_slice()),
            ("fee_payer_policy.system.create_account_only_via", create_account_only_via.as_slice()),
        ] {
            for pubkey_str in pubkeys {
                if Pubkey::from_str(pubkey_str).is_err() {
                    errors.push(format!("Invalid base58 pubkey format in {field}: '{pubkey_str}'"));
                }
            }
        }

        for (field, programs) in [
            ("require_one_of_programs", &config.validation.require_one_of_programs),
            ("fee_payer_policy.system.create_account_only_via", create_account_only_via),
        ] {
            for program in programs {
                if !config.validation.allowed_programs.contains(program) {
                    errors.push(format!(
                        "Program {program} in {field} must also be in allowed_programs"
                    ));
                }
            }
        }
    }

    /// Warn about programs in `allowed_programs` that have no dedicated fee-payer
    /// instruction parser.
    ///
    /// Two tiers:
    /// 1. **High-risk native programs** (Vote, Stake) — these can directly control funds
    ///    without routing through inner System/SPL instructions, so a targeted warning is
    ///    emitted.
    /// 2. **Any other unrecognised program** — custom programs always use inner instructions
    ///    (System, SPL Token, Token-2022, ALT, Loader-v4) for actual fund movement, so only
    ///    those inner instructions are validated. This is a known, accepted limitation; an
    ///    informational warning is emitted so operators are aware.
    pub(super) fn warn_unvalidated_programs(
        allowed_programs: &[String],
        warnings: &mut Vec<String>,
    ) {
        let high_risk = [
            (VOTE_PROGRAM_ID.to_string(), "Vote Program"),
            (STAKE_PROGRAM_ID.to_string(), "Stake Program"),
        ];

        for (program_id, program_name) in &high_risk {
            if allowed_programs.contains(program_id) {
                warnings.push(format!(
                    "{program_name} ({program_id}) is in allowed_programs but has no \
                    fee-payer instruction parser. Instructions for this program involving \
                    the fee payer will not be subject to any policy enforcement. Only add \
                    this program if you understand and accept the risk."
                ));
            }
        }

        let known_programs: HashSet<String> = [
            SYSTEM_PROGRAM_ID.to_string(),
            SPL_TOKEN_PROGRAM_ID.to_string(),
            TOKEN_2022_PROGRAM_ID.to_string(),
            solana_address_lookup_table_interface::program::ID.to_string(),
            spl_associated_token_account_interface::program::id().to_string(),
            solana_compute_budget_interface::id().to_string(),
            LIGHTHOUSE_PROGRAM_ID.to_string(),
            LOADER_V4_PROGRAM_ID.to_string(),
            BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string(),
        ]
        .into_iter()
        .chain(high_risk.iter().map(|(id, _)| id.clone()))
        .collect();

        for program_id in allowed_programs {
            if !known_programs.contains(program_id) {
                warnings.push(format!(
                    "Program {program_id} in allowed_programs has no dedicated fee-payer \
                    instruction parser. Only inner instructions to standard programs \
                    (System, SPL Token, Token-2022, ALT) will be validated for fee-payer \
                    usage. This is a known limitation for custom programs."
                ));
            }
        }
    }
}
