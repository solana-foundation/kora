use super::TransactionValidator;
use crate::{
    error::KoraError,
    transaction::{ParsedALTInstructionData, ParsedALTInstructionType},
};
use std::collections::HashMap;

impl TransactionValidator {
    pub(super) fn validate_alt_fee_payer_usage(
        &self,
        alt_instructions: &HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>,
    ) -> Result<(), KoraError> {
        deny_fee_payer!(alt_instructions, ParsedALTInstructionType::AltCreateLookupTable,
            ParsedALTInstructionData::AltCreateLookupTable {
                lookup_table_authority,
                payer_account,
                ..
            } => (*lookup_table_authority == self.fee_payer_pubkey
                || *payer_account == self.fee_payer_pubkey),
            unless self.fee_payer_policy.alt.allow_create,
            "Fee payer cannot be used for 'ALT CreateLookupTable'");

        deny_fee_payer!(alt_instructions, ParsedALTInstructionType::AltExtendLookupTable,
            ParsedALTInstructionData::AltExtendLookupTable {
                lookup_table_authority,
                payer_account,
                ..
            } => (*lookup_table_authority == self.fee_payer_pubkey
                || payer_account.is_some_and(|payer| payer == self.fee_payer_pubkey)),
            unless self.fee_payer_policy.alt.allow_extend,
            "Fee payer cannot be used for 'ALT ExtendLookupTable'");

        deny_fee_payer!(alt_instructions, ParsedALTInstructionType::AltFreezeLookupTable,
            ParsedALTInstructionData::AltFreezeLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.alt.allow_freeze,
            "Fee payer cannot be used for 'ALT FreezeLookupTable'");

        deny_fee_payer!(alt_instructions, ParsedALTInstructionType::AltDeactivateLookupTable,
            ParsedALTInstructionData::AltDeactivateLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.alt.allow_deactivate,
            "Fee payer cannot be used for 'ALT DeactivateLookupTable'");

        deny_fee_payer!(alt_instructions, ParsedALTInstructionType::AltCloseLookupTable,
            ParsedALTInstructionData::AltCloseLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            unless self.fee_payer_policy.alt.allow_close,
            "Fee payer cannot be used for 'ALT CloseLookupTable'");

        Ok(())
    }
}
