use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{ParsedSPLInstructionData, ParsedSPLInstructionType},
};
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_spl_token_fee_payer_usage(
        &self,
        spl_instructions: &HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>,
    ) -> Result<(), KoraError> {
        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenTransfer,
            ParsedSPLInstructionData::SplTokenTransfer { owner, multisig_signers, is_2022, .. } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_transfer, "Transfer");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenApprove,
            ParsedSPLInstructionData::SplTokenApprove { owner, multisig_signers, is_2022, .. } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_approve, "Approve");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenBurn,
            ParsedSPLInstructionData::SplTokenBurn { owner, multisig_signers, is_2022 } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_burn, "Burn");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenCloseAccount,
            ParsedSPLInstructionData::SplTokenCloseAccount { owner, multisig_signers, is_2022, .. } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_close_account,
            "Close Account");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenRevoke,
            ParsedSPLInstructionData::SplTokenRevoke { owner, multisig_signers, is_2022 } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_revoke, "Revoke");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenSetAuthority,
            ParsedSPLInstructionData::SplTokenSetAuthority {
                authority, new_authority, multisig_signers, is_2022
            } => (self.fee_payer_signs(authority, multisig_signers)
                || *new_authority == Some(self.fee_payer_pubkey)),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_set_authority,
            "SetAuthority");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenMintTo,
            ParsedSPLInstructionData::SplTokenMintTo { mint_authority, multisig_signers, is_2022 } =>
            self.fee_payer_signs(mint_authority, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_mint_to, "MintTo");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenInitializeMint,
            ParsedSPLInstructionData::SplTokenInitializeMint {
                mint_authority, freeze_authority, is_2022
            } => (*mint_authority == self.fee_payer_pubkey
                || *freeze_authority == Some(self.fee_payer_pubkey)),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_initialize_mint,
            "InitializeMint");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenInitializeAccount,
            ParsedSPLInstructionData::SplTokenInitializeAccount { owner, is_2022 } =>
            *owner == self.fee_payer_pubkey,
            unless token_policy(self.fee_payer_policy, *is_2022).allow_initialize_account,
            "InitializeAccount");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenInitializeMultisig,
            ParsedSPLInstructionData::SplTokenInitializeMultisig { signers, is_2022 } =>
            signers.contains(&self.fee_payer_pubkey),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_initialize_multisig,
            "InitializeMultisig");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenFreezeAccount,
            ParsedSPLInstructionData::SplTokenFreezeAccount { freeze_authority, multisig_signers, is_2022 } =>
            self.fee_payer_signs(freeze_authority, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_freeze_account,
            "FreezeAccount");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenThawAccount,
            ParsedSPLInstructionData::SplTokenThawAccount { freeze_authority, multisig_signers, is_2022 } =>
            self.fee_payer_signs(freeze_authority, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_thaw_account,
            "ThawAccount");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenWithdrawExcessLamports,
            ParsedSPLInstructionData::SplTokenWithdrawExcessLamports { owner, multisig_signers, is_2022 } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_withdraw_excess_lamports,
            "WithdrawExcessLamports");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenUnwrapLamports,
            ParsedSPLInstructionData::SplTokenUnwrapLamports { owner, multisig_signers, is_2022 } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_unwrap_lamports,
            "UnwrapLamports");

        Ok(())
    }
}
