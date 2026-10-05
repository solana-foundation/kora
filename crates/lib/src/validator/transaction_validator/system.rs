use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{ParsedSystemInstructionData, ParsedSystemInstructionType},
};
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

        // Owner allowlist for Assign: enforced for every owner while the participation gate is
        // inactive (historical behavior); scoped to assigns of a fee-payer-owned account once the
        // gate is active. The disallowed blocklist always applies. See
        // `validate_created_or_assigned_owner`.
        for instruction in
            system_instructions.get(&ParsedSystemInstructionType::SystemAssign).unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemAssign { owner, authority } = instruction {
                self.validate_created_or_assigned_owner(
                    owner,
                    *authority == self.fee_payer_pubkey,
                    "Assign",
                )?;
            }
        }

        deny_fee_payer!(system_instructions, ParsedSystemInstructionType::SystemAllocate,
            ParsedSystemInstructionData::SystemAllocate { account } =>
            *account == self.fee_payer_pubkey,
            unless self.fee_payer_policy.system.allow_allocate,
            "Fee payer cannot be used for 'System Allocate'");

        // allow_create_account gates Kora participating as the funder (payer), the seeded base
        // signer, or the account being created (prefund brick). The owner allowlist is enforced for
        // every owner while the participation gate is inactive (historical behavior), and scoped to
        // fee-payer-involved creates once it is active; the disallowed blocklist always applies. The
        // owner check uses the SAME participation definition as the create gate above (payer, base,
        // or new_account): otherwise a foreign payer could prefund-create Kora's own account and
        // assign it to an untrusted owner, taking ownership of the sponsored account. See
        // `validate_created_or_assigned_owner`.
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

                self.validate_created_or_assigned_owner(
                    owner,
                    fee_payer_participates,
                    "CreateAccount",
                )?;
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
}
