use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{ParsedLoaderV4InstructionData, ParsedLoaderV4InstructionType},
};
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_loader_v4_fee_payer_usage(
        &self,
        loader_v4_instructions: &HashMap<
            ParsedLoaderV4InstructionType,
            Vec<ParsedLoaderV4InstructionData>,
        >,
    ) -> Result<(), KoraError> {
        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::Write,
            ParsedLoaderV4InstructionData::Write { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.loader_v4.allow_write,
            "Fee payer cannot be used for 'Loader-v4 Write'");

        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::Copy,
            ParsedLoaderV4InstructionData::Copy { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.loader_v4.allow_copy,
            "Fee payer cannot be used for 'Loader-v4 Copy'");

        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::SetProgramLength,
            ParsedLoaderV4InstructionData::SetProgramLength { authority, recipient, .. } =>
            (*authority == self.fee_payer_pubkey
                || recipient.is_some_and(|r| r == self.fee_payer_pubkey)),
            unless self.fee_payer_policy.loader_v4.allow_set_program_length,
            "Fee payer cannot be used for 'Loader-v4 SetProgramLength'");

        // Drainage guard: when the fee payer is the SetProgramLength authority, the recipient
        // (if present) must be the fee payer. Otherwise shrink-to-zero or over-funded-growth
        // refunds would flow to an attacker-controlled account, draining Kora's rent.
        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::SetProgramLength,
            ParsedLoaderV4InstructionData::SetProgramLength { authority, recipient, .. } =>
            *authority == self.fee_payer_pubkey
                && recipient.is_some_and(|r| r != self.fee_payer_pubkey),
            "Loader-v4 SetProgramLength: when fee payer is the authority, \
             recipient must also be the fee payer (drainage guard)");

        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::Deploy,
            ParsedLoaderV4InstructionData::Deploy { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.loader_v4.allow_deploy,
            "Fee payer cannot be used for 'Loader-v4 Deploy'");

        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::Retract,
            ParsedLoaderV4InstructionData::Retract { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.loader_v4.allow_retract,
            "Fee payer cannot be used for 'Loader-v4 Retract'");

        // Both current_authority and new_authority are required signers in TransferAuthority.
        // The fee payer's transaction-level signature satisfies either slot, so guard both:
        // without the `new_authority` check, an attacker could craft a tx with
        // current_authority=attacker and new_authority=fee_payer and silently transfer program
        // authority to Kora (setting up later abuse if allow_write/allow_deploy are enabled).
        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::TransferAuthority,
            ParsedLoaderV4InstructionData::TransferAuthority {
                current_authority, new_authority, ..
            } => (*current_authority == self.fee_payer_pubkey
                || *new_authority == self.fee_payer_pubkey),
            unless self.fee_payer_policy.loader_v4.allow_transfer_authority,
            "Fee payer cannot be used for 'Loader-v4 TransferAuthority'");

        deny_fee_payer!(loader_v4_instructions, ParsedLoaderV4InstructionType::Finalize,
            ParsedLoaderV4InstructionData::Finalize { current_authority, .. } =>
            *current_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.loader_v4.allow_finalize,
            "Fee payer cannot be used for 'Loader-v4 Finalize'");

        Ok(())
    }
}
