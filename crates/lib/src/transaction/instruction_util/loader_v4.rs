use std::collections::HashMap;

use solana_loader_v4_interface::instruction::LoaderV4Instruction;
use solana_sdk::pubkey::Pubkey;

use super::IxUtils;
use crate::{
    constant::{instruction_indexes, LOADER_V4_PROGRAM_ID},
    error::KoraError,
    sanitize_error,
    transaction::VersionedTransactionResolved,
};

/// Loader-v4 instruction types parsed from a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedLoaderV4InstructionType {
    Write,
    Copy,
    SetProgramLength,
    Deploy,
    Retract,
    TransferAuthority,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedLoaderV4InstructionData {
    Write {
        program: Pubkey,
        authority: Pubkey,
        offset: u32,
    },
    Copy {
        destination_program: Pubkey,
        authority: Pubkey,
        source_program: Pubkey,
        destination_offset: u32,
        source_offset: u32,
        length: u32,
    },
    SetProgramLength {
        program: Pubkey,
        authority: Pubkey,
        recipient: Option<Pubkey>,
        new_size: u32,
    },
    Deploy {
        program: Pubkey,
        authority: Pubkey,
        source_program: Option<Pubkey>,
    },
    Retract {
        program: Pubkey,
        authority: Pubkey,
    },
    TransferAuthority {
        program: Pubkey,
        current_authority: Pubkey,
        new_authority: Pubkey,
    },
    Finalize {
        program: Pubkey,
        current_authority: Pubkey,
        next_version: Pubkey,
    },
}

instruction_type!(ParsedLoaderV4InstructionData => ParsedLoaderV4InstructionType {
    Write,
    Copy,
    SetProgramLength,
    Deploy,
    Retract,
    TransferAuthority,
    Finalize,
});

impl IxUtils {
    pub fn parse_loader_v4_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedLoaderV4InstructionType, Vec<ParsedLoaderV4InstructionData>>, KoraError>
    {
        let mut parsed_instructions: HashMap<
            ParsedLoaderV4InstructionType,
            Vec<ParsedLoaderV4InstructionData>,
        > = HashMap::new();

        for instruction in &transaction.all_instructions {
            if instruction.program_id != LOADER_V4_PROGRAM_ID {
                continue;
            }
            let key = |index: usize| instruction.accounts[index].pubkey;
            let optional_key = |index: usize| instruction.accounts.get(index).map(|a| a.pubkey);
            let account_count = instruction.accounts.len();

            let loader_ix = bincode::deserialize::<LoaderV4Instruction>(&instruction.data)
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to parse Loader-v4 instruction: {}",
                        sanitize_error!(e)
                    ))
                })?;

            let data = match loader_ix {
                LoaderV4Instruction::Write { offset, .. } => {
                    use instruction_indexes::loader_v4_write as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedLoaderV4InstructionData::Write {
                        program: key(ix::PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        offset,
                    }
                }
                LoaderV4Instruction::Copy { destination_offset, source_offset, length } => {
                    use instruction_indexes::loader_v4_copy as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedLoaderV4InstructionData::Copy {
                        destination_program: key(ix::DESTINATION_PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        source_program: key(ix::SOURCE_PROGRAM_INDEX),
                        destination_offset,
                        source_offset,
                        length,
                    }
                }
                LoaderV4Instruction::SetProgramLength { new_size } => {
                    use instruction_indexes::loader_v4_set_program_length as ix;
                    if account_count < ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Loader-v4 SetProgramLength has {account_count} accounts, expected at least 2"
                        )));
                    }
                    ParsedLoaderV4InstructionData::SetProgramLength {
                        program: key(ix::PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        recipient: optional_key(ix::OPTIONAL_RECIPIENT_INDEX),
                        new_size,
                    }
                }
                LoaderV4Instruction::Deploy => {
                    use instruction_indexes::loader_v4_deploy as ix;
                    if account_count < ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Loader-v4 Deploy has {account_count} accounts, expected at least 2"
                        )));
                    }
                    ParsedLoaderV4InstructionData::Deploy {
                        program: key(ix::PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        source_program: optional_key(ix::OPTIONAL_SOURCE_PROGRAM_INDEX),
                    }
                }
                LoaderV4Instruction::Retract => {
                    use instruction_indexes::loader_v4_retract as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedLoaderV4InstructionData::Retract {
                        program: key(ix::PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                    }
                }
                LoaderV4Instruction::TransferAuthority => {
                    use instruction_indexes::loader_v4_transfer_authority as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedLoaderV4InstructionData::TransferAuthority {
                        program: key(ix::PROGRAM_INDEX),
                        current_authority: key(ix::CURRENT_AUTHORITY_INDEX),
                        new_authority: key(ix::NEW_AUTHORITY_INDEX),
                    }
                }
                LoaderV4Instruction::Finalize => {
                    use instruction_indexes::loader_v4_finalize as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedLoaderV4InstructionData::Finalize {
                        program: key(ix::PROGRAM_INDEX),
                        current_authority: key(ix::CURRENT_AUTHORITY_INDEX),
                        next_version: key(ix::NEXT_VERSION_INDEX),
                    }
                }
            };
            parsed_instructions.entry(data.instruction_type()).or_default().push(data);
        }

        Ok(parsed_instructions)
    }
}
