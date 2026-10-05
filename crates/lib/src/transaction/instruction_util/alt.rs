use std::collections::HashMap;

use solana_address_lookup_table_interface::{
    instruction::ProgramInstruction as AddressLookupTableInstruction,
    program::ID as ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;

use super::IxUtils;
use crate::{
    constant::instruction_indexes, error::KoraError, sanitize_error,
    transaction::VersionedTransactionResolved,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedALTInstructionType {
    AltCreateLookupTable,
    AltExtendLookupTable,
    AltFreezeLookupTable,
    AltDeactivateLookupTable,
    AltCloseLookupTable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedALTInstructionData {
    AltCreateLookupTable {
        lookup_table_account: Pubkey,
        lookup_table_authority: Pubkey,
        payer_account: Pubkey,
    },
    AltExtendLookupTable {
        lookup_table_account: Pubkey,
        lookup_table_authority: Pubkey,
        payer_account: Option<Pubkey>,
    },
    AltFreezeLookupTable {
        lookup_table_account: Pubkey,
        lookup_table_authority: Pubkey,
    },
    AltDeactivateLookupTable {
        lookup_table_account: Pubkey,
        lookup_table_authority: Pubkey,
    },
    AltCloseLookupTable {
        lookup_table_account: Pubkey,
        lookup_table_authority: Pubkey,
        recipient: Pubkey,
    },
}

instruction_type!(ParsedALTInstructionData => ParsedALTInstructionType {
    AltCreateLookupTable,
    AltExtendLookupTable,
    AltFreezeLookupTable,
    AltDeactivateLookupTable,
    AltCloseLookupTable,
});

impl IxUtils {
    pub fn parse_alt_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>, KoraError> {
        let mut parsed_instructions: HashMap<
            ParsedALTInstructionType,
            Vec<ParsedALTInstructionData>,
        > = HashMap::new();

        for instruction in &transaction.all_instructions {
            if instruction.program_id != ADDRESS_LOOKUP_TABLE_PROGRAM_ID {
                continue;
            }
            let key = |index: usize| instruction.accounts[index].pubkey;

            let alt_ix = bincode::deserialize::<AddressLookupTableInstruction>(&instruction.data)
                .map_err(|e| {
                KoraError::InvalidTransaction(format!(
                    "Failed to parse ALT instruction: {}",
                    sanitize_error!(e)
                ))
            })?;

            let data = match alt_ix {
                AddressLookupTableInstruction::CreateLookupTable { .. } => {
                    use instruction_indexes::alt_create_lookup_table as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedALTInstructionData::AltCreateLookupTable {
                        lookup_table_account: key(ix::LOOKUP_TABLE_ACCOUNT_INDEX),
                        lookup_table_authority: key(ix::LOOKUP_TABLE_AUTHORITY_INDEX),
                        payer_account: key(ix::PAYER_ACCOUNT_INDEX),
                    }
                }
                AddressLookupTableInstruction::ExtendLookupTable { .. } => {
                    use instruction_indexes::alt_extend_lookup_table as ix;
                    let account_count = instruction.accounts.len();
                    if account_count < ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS || account_count == 3 {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Instruction account mismatch: expected 2 or >=4 accounts, found {account_count}"
                        )));
                    }
                    ParsedALTInstructionData::AltExtendLookupTable {
                        lookup_table_account: key(ix::LOOKUP_TABLE_ACCOUNT_INDEX),
                        lookup_table_authority: key(ix::LOOKUP_TABLE_AUTHORITY_INDEX),
                        payer_account: (account_count
                            >= ix::REQUIRED_NUMBER_OF_ACCOUNTS_WITH_PAYER)
                            .then(|| key(ix::OPTIONAL_PAYER_ACCOUNT_INDEX)),
                    }
                }
                AddressLookupTableInstruction::FreezeLookupTable => {
                    use instruction_indexes::alt_freeze_lookup_table as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedALTInstructionData::AltFreezeLookupTable {
                        lookup_table_account: key(ix::LOOKUP_TABLE_ACCOUNT_INDEX),
                        lookup_table_authority: key(ix::LOOKUP_TABLE_AUTHORITY_INDEX),
                    }
                }
                AddressLookupTableInstruction::DeactivateLookupTable => {
                    use instruction_indexes::alt_deactivate_lookup_table as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedALTInstructionData::AltDeactivateLookupTable {
                        lookup_table_account: key(ix::LOOKUP_TABLE_ACCOUNT_INDEX),
                        lookup_table_authority: key(ix::LOOKUP_TABLE_AUTHORITY_INDEX),
                    }
                }
                AddressLookupTableInstruction::CloseLookupTable => {
                    use instruction_indexes::alt_close_lookup_table as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedALTInstructionData::AltCloseLookupTable {
                        lookup_table_account: key(ix::LOOKUP_TABLE_ACCOUNT_INDEX),
                        lookup_table_authority: key(ix::LOOKUP_TABLE_AUTHORITY_INDEX),
                        recipient: key(ix::RECIPIENT_INDEX),
                    }
                }
            };
            parsed_instructions.entry(data.instruction_type()).or_default().push(data);
        }

        Ok(parsed_instructions)
    }
}
