use std::collections::HashMap;

use solana_loader_v3_interface::instruction::UpgradeableLoaderInstruction;
use solana_sdk::pubkey::Pubkey;

use super::IxUtils;
use crate::{
    constant::{instruction_indexes, BPF_LOADER_UPGRADEABLE_PROGRAM_ID},
    error::KoraError,
    sanitize_error,
    transaction::VersionedTransactionResolved,
};

/// BPF Loader Upgradeable (loader-v3) instruction types parsed from a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedBpfLoaderUpgradeableInstructionType {
    InitializeBuffer,
    Write,
    DeployWithMaxDataLen,
    Upgrade,
    SetAuthority,
    SetAuthorityChecked,
    Close,
    ExtendProgram,
    ExtendProgramChecked,
    Migrate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedBpfLoaderUpgradeableInstructionData {
    InitializeBuffer {
        buffer: Pubkey,
        authority: Option<Pubkey>,
    },
    Write {
        buffer: Pubkey,
        authority: Pubkey,
        offset: u32,
    },
    DeployWithMaxDataLen {
        payer: Pubkey,
        program_data: Pubkey,
        program: Pubkey,
        buffer: Pubkey,
        upgrade_authority: Pubkey,
        max_data_len: u64,
    },
    Upgrade {
        program_data: Pubkey,
        program: Pubkey,
        buffer: Pubkey,
        spill: Pubkey,
        upgrade_authority: Pubkey,
    },
    SetAuthority {
        target: Pubkey,
        current_authority: Pubkey,
        new_authority: Option<Pubkey>,
    },
    SetAuthorityChecked {
        target: Pubkey,
        current_authority: Pubkey,
        new_authority: Pubkey,
    },
    Close {
        target: Pubkey,
        recipient: Pubkey,
        authority: Option<Pubkey>,
        program: Option<Pubkey>,
    },
    ExtendProgram {
        program_data: Pubkey,
        program: Pubkey,
        payer: Option<Pubkey>,
        additional_bytes: u32,
    },
    ExtendProgramChecked {
        program_data: Pubkey,
        program: Pubkey,
        authority: Pubkey,
        payer: Option<Pubkey>,
        additional_bytes: u32,
    },
    Migrate {
        program_data: Pubkey,
        program: Pubkey,
        current_authority: Pubkey,
    },
}

instruction_type!(
    ParsedBpfLoaderUpgradeableInstructionData => ParsedBpfLoaderUpgradeableInstructionType {
        InitializeBuffer,
        Write,
        DeployWithMaxDataLen,
        Upgrade,
        SetAuthority,
        SetAuthorityChecked,
        Close,
        ExtendProgram,
        ExtendProgramChecked,
        Migrate,
    }
);

impl IxUtils {
    pub fn parse_bpf_loader_upgradeable_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<
        HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
        KoraError,
    > {
        let mut parsed_instructions: HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        > = HashMap::new();

        for instruction in &transaction.all_instructions {
            if instruction.program_id != BPF_LOADER_UPGRADEABLE_PROGRAM_ID {
                continue;
            }
            let key = |index: usize| instruction.accounts[index].pubkey;
            let optional_key = |index: usize| instruction.accounts.get(index).map(|a| a.pubkey);
            let account_count = instruction.accounts.len();
            let require_min = |min: usize, name: &str| {
                if account_count < min {
                    return Err(KoraError::InvalidTransaction(format!(
                        "BPF Loader Upgradeable {name} has {account_count} accounts, expected at least {min}"
                    )));
                }
                Ok(())
            };

            let parsed = bincode::deserialize::<UpgradeableLoaderInstruction>(&instruction.data)
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to parse BPF Loader Upgradeable instruction: {}",
                        sanitize_error!(e)
                    ))
                })?;

            let data = match parsed {
                UpgradeableLoaderInstruction::InitializeBuffer => {
                    use instruction_indexes::bpf_loader_upgradeable_initialize_buffer as ix;
                    require_min(ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS, "InitializeBuffer")?;
                    ParsedBpfLoaderUpgradeableInstructionData::InitializeBuffer {
                        buffer: key(ix::BUFFER_INDEX),
                        authority: optional_key(ix::OPTIONAL_AUTHORITY_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::Write { offset, .. } => {
                    use instruction_indexes::bpf_loader_upgradeable_write as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedBpfLoaderUpgradeableInstructionData::Write {
                        buffer: key(ix::BUFFER_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        offset,
                    }
                }
                UpgradeableLoaderInstruction::DeployWithMaxDataLen { max_data_len } => {
                    use instruction_indexes::bpf_loader_upgradeable_deploy_with_max_data_len as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedBpfLoaderUpgradeableInstructionData::DeployWithMaxDataLen {
                        payer: key(ix::PAYER_INDEX),
                        program_data: key(ix::PROGRAM_DATA_INDEX),
                        program: key(ix::PROGRAM_INDEX),
                        buffer: key(ix::BUFFER_INDEX),
                        upgrade_authority: key(ix::UPGRADE_AUTHORITY_INDEX),
                        max_data_len: max_data_len as u64,
                    }
                }
                UpgradeableLoaderInstruction::Upgrade => {
                    use instruction_indexes::bpf_loader_upgradeable_upgrade as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedBpfLoaderUpgradeableInstructionData::Upgrade {
                        program_data: key(ix::PROGRAM_DATA_INDEX),
                        program: key(ix::PROGRAM_INDEX),
                        buffer: key(ix::BUFFER_INDEX),
                        spill: key(ix::SPILL_INDEX),
                        upgrade_authority: key(ix::UPGRADE_AUTHORITY_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::SetAuthority => {
                    use instruction_indexes::bpf_loader_upgradeable_set_authority as ix;
                    require_min(ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS, "SetAuthority")?;
                    ParsedBpfLoaderUpgradeableInstructionData::SetAuthority {
                        target: key(ix::TARGET_INDEX),
                        current_authority: key(ix::CURRENT_AUTHORITY_INDEX),
                        new_authority: optional_key(ix::OPTIONAL_NEW_AUTHORITY_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::SetAuthorityChecked => {
                    use instruction_indexes::bpf_loader_upgradeable_set_authority_checked as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedBpfLoaderUpgradeableInstructionData::SetAuthorityChecked {
                        target: key(ix::TARGET_INDEX),
                        current_authority: key(ix::CURRENT_AUTHORITY_INDEX),
                        new_authority: key(ix::NEW_AUTHORITY_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::Close => {
                    use instruction_indexes::bpf_loader_upgradeable_close as ix;
                    require_min(ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS, "Close")?;
                    ParsedBpfLoaderUpgradeableInstructionData::Close {
                        target: key(ix::TARGET_INDEX),
                        recipient: key(ix::RECIPIENT_INDEX),
                        authority: optional_key(ix::OPTIONAL_AUTHORITY_INDEX),
                        program: optional_key(ix::OPTIONAL_PROGRAM_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::ExtendProgram { additional_bytes } => {
                    use instruction_indexes::bpf_loader_upgradeable_extend_program as ix;
                    require_min(ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS, "ExtendProgram")?;
                    ParsedBpfLoaderUpgradeableInstructionData::ExtendProgram {
                        program_data: key(ix::PROGRAM_DATA_INDEX),
                        program: key(ix::PROGRAM_INDEX),
                        payer: optional_key(ix::OPTIONAL_PAYER_INDEX),
                        additional_bytes,
                    }
                }
                UpgradeableLoaderInstruction::Migrate => {
                    use instruction_indexes::bpf_loader_upgradeable_migrate as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedBpfLoaderUpgradeableInstructionData::Migrate {
                        program_data: key(ix::PROGRAM_DATA_INDEX),
                        program: key(ix::PROGRAM_INDEX),
                        current_authority: key(ix::CURRENT_AUTHORITY_INDEX),
                    }
                }
                UpgradeableLoaderInstruction::ExtendProgramChecked { additional_bytes } => {
                    use instruction_indexes::bpf_loader_upgradeable_extend_program_checked as ix;
                    require_min(ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS, "ExtendProgramChecked")?;
                    ParsedBpfLoaderUpgradeableInstructionData::ExtendProgramChecked {
                        program_data: key(ix::PROGRAM_DATA_INDEX),
                        program: key(ix::PROGRAM_INDEX),
                        authority: key(ix::AUTHORITY_INDEX),
                        payer: optional_key(ix::OPTIONAL_PAYER_INDEX),
                        additional_bytes,
                    }
                }
            };
            parsed_instructions.entry(data.instruction_type()).or_default().push(data);
        }

        Ok(parsed_instructions)
    }
}
