use super::TransactionValidator;
use crate::{
    cache::CacheUtil,
    config::Config,
    error::KoraError,
    token::token::TokenType,
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
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
            ParsedSPLInstructionData::SplTokenBurn { owner, multisig_signers, is_2022, .. } =>
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
                authority, new_authority, multisig_signers, is_2022, ..
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
            ParsedSPLInstructionData::SplTokenWithdrawExcessLamports { owner, multisig_signers, is_2022, .. } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_withdraw_excess_lamports,
            "WithdrawExcessLamports");

        deny_fee_payer!(spl_instructions, ParsedSPLInstructionType::SplTokenUnwrapLamports,
            ParsedSPLInstructionData::SplTokenUnwrapLamports { owner, multisig_signers, is_2022, .. } =>
            self.fee_payer_signs(owner, multisig_signers),
            unless token_policy(self.fee_payer_policy, *is_2022).allow_unwrap_lamports,
            "UnwrapLamports");

        Ok(())
    }

    /// Not flag-gated: `allow_close_account` only covers token accounts Kora is the close
    /// authority of but does not own. Closing an account owned by the fee payer or the payment
    /// address frees its address, and a caller holding that address's keypair can then recreate
    /// a payment destination Kora already priced with another mint and owner.
    pub(super) async fn validate_fee_payer_token_account_closes(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        let closes: Vec<Pubkey> = transaction_resolved
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenCloseAccount)
            .into_iter()
            .flatten()
            .filter_map(|instruction| match instruction {
                ParsedSPLInstructionData::SplTokenCloseAccount {
                    owner,
                    account,
                    multisig_signers,
                    ..
                } if self.fee_payer_signs(owner, multisig_signers) => Some(*account),
                _ => None,
            })
            .collect();
        if closes.is_empty() {
            return Ok(());
        }

        let payment_address = config.kora.get_payment_address(&self.fee_payer_pubkey)?;
        for account in closes {
            let account_data = CacheUtil::get_account(config, rpc_client, &account, true).await?;
            let token_program = TokenType::get_token_program_from_owner(&account_data.owner)?;
            let owner = token_program
                .unpack_token_account(&account_data.data)
                .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {e}")))?
                .owner();
            if owner == self.fee_payer_pubkey || owner == payment_address {
                return Err(KoraError::InvalidTransaction(format!(
                    "Fee payer cannot close token account {account} owned by Kora"
                )));
            }
        }

        Ok(())
    }
}
