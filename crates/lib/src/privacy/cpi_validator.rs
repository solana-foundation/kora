//! CPI-based fee payment validation.
//!
//! This module provides validation logic for fee payments made via CPI
//! (Cross-Program Invocation) from privacy pool programs.
//!
//! # Overview
//!
//! Standard Kora validates top-level SPL token transfers. This module extends
//! that to also validate CPI transfers that:
//!
//! 1. Originate from an allowed program (the privacy pool)
//! 2. Transfer tokens to the expected payment destination (relayer)
//! 3. Transfer sufficient value to cover the required fee
//!
//! # Security Model
//!
//! The key security property is that we only accept fee payments from CPIs
//! initiated by programs in the `allowed_fee_payment_programs` list. This
//! prevents arbitrary programs from claiming to have paid the fee.

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status_client_types::UiInnerInstructions;

use crate::{
    cache::CacheUtil,
    config::Config,
    error::KoraError,
    token::{
        interface::TokenInterface,
        spl_token::TokenProgram,
        spl_token_2022::Token2022Program,
        token::TokenUtil,
    },
    transaction::{ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved},
};

use super::{
    config::PrivacyConfig,
    instruction_context::{get_outer_instruction_index, InnerInstructionMap},
};

/// Result of CPI payment validation.
#[derive(Debug, Clone)]
pub struct CpiPaymentResult {
    /// Whether sufficient payment was found.
    pub payment_found: bool,
    /// Total payment value in lamports.
    pub total_lamports: u64,
    /// Number of valid payment transfers found.
    pub payment_count: usize,
}

impl CpiPaymentResult {
    /// Create a result indicating no payment was found.
    pub fn no_payment() -> Self {
        Self {
            payment_found: false,
            total_lamports: 0,
            payment_count: 0,
        }
    }

    /// Create a result with the given payment details.
    pub fn with_payment(total_lamports: u64, payment_count: usize, required_lamports: u64) -> Self {
        Self {
            payment_found: total_lamports >= required_lamports,
            total_lamports,
            payment_count,
        }
    }
}

/// Validator for CPI-based fee payments from privacy pool programs.
pub struct CpiPaymentValidator<'a> {
    privacy_config: &'a PrivacyConfig,
}

impl<'a> CpiPaymentValidator<'a> {
    /// Create a new CPI payment validator.
    pub fn new(privacy_config: &'a PrivacyConfig) -> Self {
        Self { privacy_config }
    }

    /// Check if privacy mode is active and should be used.
    pub fn is_active(&self) -> bool {
        self.privacy_config.is_active()
    }

    /// Verify that valid CPI-based fee payment exists in the transaction.
    ///
    /// This checks for SPL token transfers in inner instructions that:
    /// 1. Come from an allowed program (privacy pool)
    /// 2. Transfer to the expected payment destination (relayer)
    /// 3. Transfer sufficient value in aggregate
    ///
    /// # Arguments
    ///
    /// * `config` - Kora configuration
    /// * `transaction_resolved` - The resolved transaction with parsed instructions
    /// * `rpc_client` - RPC client for account lookups
    /// * `required_lamports` - Minimum payment required in lamports equivalent
    /// * `expected_destination_owner` - Expected owner of the payment destination account
    /// * `inner_instructions` - Inner instructions from transaction simulation
    ///
    /// # Returns
    ///
    /// `CpiPaymentResult` with payment validation details.
    pub async fn verify_cpi_payment(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        required_lamports: u64,
        expected_destination_owner: &Pubkey,
        inner_instructions: Option<&[UiInnerInstructions]>,
    ) -> Result<CpiPaymentResult, KoraError> {
        if !self.is_active() {
            return Ok(CpiPaymentResult::no_payment());
        }

        // If no inner instructions, there can't be any CPI payments
        let inner_ixs = match inner_instructions {
            Some(ixs) if !ixs.is_empty() => ixs,
            _ => return Ok(CpiPaymentResult::no_payment()),
        };

        // Build map of outer instruction programs
        let outer_instruction_map =
            InnerInstructionMap::from_outer_instructions(&transaction_resolved.all_instructions);

        let allowed_programs = self.privacy_config.get_allowed_programs();

        // Track which inner instruction groups come from allowed programs
        let allowed_groups: Vec<usize> = inner_ixs
            .iter()
            .filter_map(|group| {
                let outer_index = get_outer_instruction_index(group);
                if outer_instruction_map.is_from_allowed_program(outer_index, allowed_programs) {
                    Some(outer_index)
                } else {
                    None
                }
            })
            .collect();

        if allowed_groups.is_empty() {
            log::debug!("No inner instructions from allowed programs");
            return Ok(CpiPaymentResult::no_payment());
        }

        log::debug!(
            "Found {} inner instruction groups from allowed programs",
            allowed_groups.len()
        );

        // Now check parsed SPL instructions for transfers to payment destination
        // The existing parsing already includes inner instructions
        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;

        let transfers = spl_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let mut total_lamports: u64 = 0;
        let mut payment_count: usize = 0;

        for transfer in transfers {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                destination_address,
                amount,
                is_2022,
                ..
            } = transfer
            {
                // Get token program for unpacking account data
                let token_program: Box<dyn TokenInterface> = if *is_2022 {
                    Box::new(Token2022Program::new())
                } else {
                    Box::new(TokenProgram::new())
                };

                // Verify destination account owner matches expected payment address
                let destination_account =
                    match CacheUtil::get_account(config, rpc_client, destination_address, false)
                        .await
                    {
                        Ok(account) => account,
                        Err(KoraError::AccountNotFound(_)) => {
                            // Destination doesn't exist - skip this transfer
                            continue;
                        }
                        Err(e) => return Err(e),
                    };

                let token_state = token_program
                    .unpack_token_account(&destination_account.data)
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                    })?;

                // Skip if destination owner doesn't match expected payment address
                if token_state.owner() != *expected_destination_owner {
                    continue;
                }

                let token_mint = token_state.mint();

                // Check if token is supported
                if !config.validation.supports_token(&token_mint.to_string()) {
                    log::debug!(
                        "Ignoring CPI payment with unsupported token mint: {}",
                        token_mint
                    );
                    continue;
                }

                // Calculate value in lamports
                let lamport_value =
                    TokenUtil::calculate_token_value_in_lamports(*amount, &token_mint, rpc_client, config)
                        .await?;

                total_lamports = total_lamports.checked_add(lamport_value).ok_or_else(|| {
                    log::error!(
                        "CPI payment accumulation overflow: total={}, new_payment={}",
                        total_lamports,
                        lamport_value
                    );
                    KoraError::ValidationError("CPI payment accumulation overflow".to_string())
                })?;

                payment_count += 1;

                log::debug!(
                    "Found valid CPI payment: {} tokens = {} lamports",
                    amount,
                    lamport_value
                );
            }
        }

        Ok(CpiPaymentResult::with_payment(
            total_lamports,
            payment_count,
            required_lamports,
        ))
    }

    /// Simple check if any transfers to payment destination exist from allowed programs.
    ///
    /// This is a lighter-weight check that doesn't calculate token values.
    /// Use `verify_cpi_payment` for full validation with value calculation.
    pub fn has_payment_transfer_from_allowed_program(
        &self,
        outer_instructions: &[solana_sdk::instruction::Instruction],
        inner_instructions: &[UiInnerInstructions],
    ) -> bool {
        if !self.is_active() {
            return false;
        }

        let map = InnerInstructionMap::from_outer_instructions(outer_instructions);
        let allowed = self.privacy_config.get_allowed_programs();

        inner_instructions.iter().any(|group| {
            let outer_index = get_outer_instruction_index(group);
            map.is_from_allowed_program(outer_index, allowed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_privacy_config(enabled: bool, programs: Vec<Pubkey>) -> PrivacyConfig {
        let mut config = PrivacyConfig::default();
        config.enabled = enabled;
        config.allowed_fee_payment_programs = programs.iter().map(|p| p.to_string()).collect();
        config.initialize().unwrap();
        config
    }

    #[test]
    fn test_cpi_payment_result_no_payment() {
        let result = CpiPaymentResult::no_payment();
        assert!(!result.payment_found);
        assert_eq!(result.total_lamports, 0);
        assert_eq!(result.payment_count, 0);
    }

    #[test]
    fn test_cpi_payment_result_with_sufficient_payment() {
        let result = CpiPaymentResult::with_payment(1000, 1, 500);
        assert!(result.payment_found);
        assert_eq!(result.total_lamports, 1000);
        assert_eq!(result.payment_count, 1);
    }

    #[test]
    fn test_cpi_payment_result_with_insufficient_payment() {
        let result = CpiPaymentResult::with_payment(100, 1, 500);
        assert!(!result.payment_found);
        assert_eq!(result.total_lamports, 100);
        assert_eq!(result.payment_count, 1);
    }

    #[test]
    fn test_validator_inactive_when_disabled() {
        let config = make_test_privacy_config(false, vec![Pubkey::new_unique()]);
        let validator = CpiPaymentValidator::new(&config);
        assert!(!validator.is_active());
    }

    #[test]
    fn test_validator_inactive_when_no_programs() {
        let config = make_test_privacy_config(true, vec![]);
        let validator = CpiPaymentValidator::new(&config);
        assert!(!validator.is_active());
    }

    #[test]
    fn test_validator_active_when_enabled_with_programs() {
        let config = make_test_privacy_config(true, vec![Pubkey::new_unique()]);
        let validator = CpiPaymentValidator::new(&config);
        assert!(validator.is_active());
    }

    #[test]
    fn test_has_payment_transfer_returns_false_when_inactive() {
        let config = make_test_privacy_config(false, vec![]);
        let validator = CpiPaymentValidator::new(&config);

        let outer_instructions = vec![];
        let inner_instructions = vec![];

        assert!(!validator.has_payment_transfer_from_allowed_program(
            &outer_instructions,
            &inner_instructions
        ));
    }
}
