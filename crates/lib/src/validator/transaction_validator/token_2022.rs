use super::TransactionValidator;
use crate::{
    config::Config,
    error::KoraError,
    token::token::{TokenUtil, TransferHookValidationFlow},
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, Token2022AccountUsagePolicy,
        VersionedTransactionResolved,
    },
};
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_token2022_fee_payer_usage(
        &self,
        spl_instructions: &HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>,
    ) -> Result<(), KoraError> {
        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenReallocate,
            ParsedSPLInstructionData::SplTokenReallocate {
                payer,
                owner,
                multisig_signers,
                is_2022,
                ..
            } => *is_2022
                && (*payer == self.fee_payer_pubkey
                    || self.fee_payer_signs(owner, multisig_signers)),
            "Token2022 Reallocate is not allowed when involving fee payer");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenPause,
            ParsedSPLInstructionData::SplTokenPause { authority, multisig_signers } =>
            self.fee_payer_signs(authority, multisig_signers),
            unless self.fee_payer_policy.token_2022.allow_freeze_account,
            "Fee payer cannot be used for Token2022 Pause");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenResume,
            ParsedSPLInstructionData::SplTokenResume { authority, multisig_signers } =>
            self.fee_payer_signs(authority, multisig_signers),
            unless self.fee_payer_policy.token_2022.allow_thaw_account,
            "Fee payer cannot be used for Token2022 Resume");

        Ok(())
    }

    pub(crate) fn validate_token2022_transfer_hook_signing_policies(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        transfer_hook_validation_flow: TransferHookValidationFlow,
    ) -> Result<(), KoraError> {
        if !TokenUtil::should_reject_mutable_transfer_hook(config, transfer_hook_validation_flow) {
            return Ok(());
        }

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenInitializeTransferHook,
            ParsedSPLInstructionData::SplTokenInitializeTransferHook {
                authority: Some(authority),
                ..
            } => *authority == self.fee_payer_pubkey,
            "Fee payer cannot initialize mutable Token2022 TransferHook authority");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenTransferHookUpdate,
            ParsedSPLInstructionData::SplTokenTransferHookUpdate {
                authority,
                multisig_signers,
                ..
            } => self.fee_payer_signs(authority, multisig_signers),
            "Fee payer cannot authorize mutable Token2022 TransferHook updates");

        Ok(())
    }

    pub(super) fn validate_token2022_extension_security(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        for instruction in transaction_resolved.get_or_parse_token2022_security_instructions()? {
            if matches!(
                instruction.account_usage_policy,
                Token2022AccountUsagePolicy::RejectIfFeePayerPresent
            ) && instruction.accounts.contains(&self.fee_payer_pubkey)
            {
                return Err(KoraError::InvalidTransaction(format!(
                    "Fee payer cannot be an account in {}",
                    instruction.instruction_name
                )));
            }

            if let Some(extension_type) = instruction.extension_type {
                if config.validation.token_2022.is_mint_extension_blocked(extension_type)
                    || config.validation.token_2022.is_account_extension_blocked(extension_type)
                {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Token2022 instruction '{}' is not allowed because extension '{extension_type:?}' is blocked",
                        instruction.instruction_name
                    )));
                }
            }

            if instruction.uses_fee_payer_as_current_extension_authority(&self.fee_payer_pubkey)
                && !self.fee_payer_policy.token_2022.allow_update_extension_authority
            {
                return Err(KoraError::InvalidTransaction(format!(
                    "Fee payer cannot be used as the current Token2022 extension authority for '{}'",
                    instruction.instruction_name
                )));
            }

            if let Some(field) =
                instruction.find_planted_fee_payer_authority(&self.fee_payer_pubkey)
            {
                if !self.fee_payer_policy.token_2022.allow_initialize_extension_authority {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be planted as a Token2022 extension authority via {}",
                        field.context
                    )));
                }
            }
        }

        Ok(())
    }
}
