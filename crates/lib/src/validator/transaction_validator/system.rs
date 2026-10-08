use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{ParsedSystemInstructionData, ParsedSystemInstructionType},
};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_system_fee_payer_usage(
        &self,
        system_instructions: &HashMap<
            ParsedSystemInstructionType,
            Vec<ParsedSystemInstructionData>,
        >,
    ) -> Result<(), KoraError> {
        if !self.allow_durable_transactions
            && system_instructions
                .contains_key(&ParsedSystemInstructionType::SystemAdvanceNonceAccount)
        {
            return Err(KoraError::InvalidTransaction(
                "Durable transactions (nonce-based) are not allowed".to_string(),
            ));
        }

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemTransfer,
            ParsedSystemInstructionData::SystemTransfer { sender, .. } =>
            *sender == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.allow_transfer,
            "Fee payer cannot be used for 'System Transfer'");

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemAssign,
            ParsedSystemInstructionData::SystemAssign { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.allow_assign,
            "Fee payer cannot be used for 'System Assign'");

        // The owner allowlist/disallowed check holds for every Assign owner in a Kora-signed tx,
        // not only when the fee payer is the reassigned account (parity with CreateAccount).
        for instruction in
            system_instructions.get(&ParsedSystemInstructionType::SystemAssign).unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemAssign { owner, .. } = instruction {
                self.validate_owner_program("Assign", owner)?;
            }
        }

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemAllocate,
            ParsedSystemInstructionData::SystemAllocate { account } =>
            *account == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.allow_allocate,
            "Fee payer cannot be used for 'System Allocate'");

        // allow_create_account gates Kora participating as the funder (payer), the seeded base
        // signer, or the account being created (prefund brick). The owner allowlist/blocklist
        // holds for every CreateAccount owner in a Kora-signed tx regardless of Kora's role.
        for instruction in system_instructions
            .get(&ParsedSystemInstructionType::SystemCreateAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemCreateAccount {
                payer,
                owner,
                base,
                new_account,
                ..
            } = instruction
            {
                let fee_payer_participates = *payer == self.fee_payer_pubkey
                    || *base == Some(self.fee_payer_pubkey)
                    || *new_account == self.fee_payer_pubkey;
                if fee_payer_participates && !self.fee_payer_policy.system.allow_create_account {
                    return Err(KoraError::InvalidTransaction(
                        "Fee payer cannot be used for 'System Create Account'".to_string(),
                    ));
                }

                self.validate_owner_program("CreateAccount", owner)?;
            }
        }

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemInitializeNonceAccount,
            ParsedSystemInstructionData::SystemInitializeNonceAccount { nonce_authority, .. } =>
            *nonce_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.nonce.allow_initialize,
            "Fee payer cannot be used for 'System Initialize Nonce Account'");

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemAdvanceNonceAccount,
            ParsedSystemInstructionData::SystemAdvanceNonceAccount { nonce_authority, .. } =>
            *nonce_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.nonce.allow_advance,
            "Fee payer cannot be used for 'System Advance Nonce Account'");

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemAuthorizeNonceAccount,
            ParsedSystemInstructionData::SystemAuthorizeNonceAccount { nonce_authority, .. } =>
            *nonce_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.nonce.allow_authorize,
            "Fee payer cannot be used for 'System Authorize Nonce Account'");

        // Note: SystemUpgradeNonceAccount not validated - no authority parameter

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemWithdrawNonceAccount,
            ParsedSystemInstructionData::SystemWithdrawNonceAccount { nonce_authority, .. } =>
            *nonce_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.nonce.allow_withdraw,
            "Fee payer cannot be used for 'System Withdraw Nonce Account'");

        Ok(())
    }

    fn validate_owner_program(&self, instruction: &str, owner: &Pubkey) -> Result<(), KoraError> {
        if !self.allow_all_programs && !self.allowed_programs.contains(owner) {
            return Err(KoraError::InvalidTransaction(format!(
                "{instruction} owner program {owner} is not in the allowed programs list"
            )));
        }
        if self.disallowed_accounts.contains(owner) {
            return Err(KoraError::InvalidTransaction(format!(
                "{instruction} owner program {owner} is in the disallowed accounts list"
            )));
        }
        Ok(())
    }
}
