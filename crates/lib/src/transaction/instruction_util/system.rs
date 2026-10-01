use std::collections::HashMap;

use solana_sdk::pubkey::Pubkey;
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};

use super::IxUtils;
use crate::{
    constant::instruction_indexes, error::KoraError, transaction::VersionedTransactionResolved,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionType {
    SystemTransfer,
    SystemCreateAccount,
    SystemWithdrawNonceAccount,
    SystemAssign,
    SystemAllocate,
    SystemInitializeNonceAccount,
    SystemAdvanceNonceAccount,
    SystemAuthorizeNonceAccount,
    // Note: SystemUpgradeNonceAccount not included - no authority parameter
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionData {
    // Includes transfer and transfer with seed
    SystemTransfer {
        lamports: u64,
        sender: Pubkey,
        receiver: Pubkey,
    },
    // Includes create account and create account with seed
    SystemCreateAccount {
        lamports: u64,
        payer: Pubkey,
        new_account: Pubkey,
        owner: Pubkey,
        // CreateAccountWithSeed has a distinct required `base` signer; None for plain CreateAccount.
        base: Option<Pubkey>,
    },
    SystemWithdrawNonceAccount {
        lamports: u64,
        nonce_authority: Pubkey,
        recipient: Pubkey,
    },
    // Includes assign and assign with seed
    SystemAssign {
        authority: Pubkey,
        owner: Pubkey,
    },
    // Includes allocate and allocate with seed
    SystemAllocate {
        account: Pubkey,
    },
    SystemInitializeNonceAccount {
        nonce_account: Pubkey,
        nonce_authority: Pubkey,
    },
    SystemAdvanceNonceAccount {
        nonce_account: Pubkey,
        nonce_authority: Pubkey,
    },
    SystemAuthorizeNonceAccount {
        nonce_account: Pubkey,
        nonce_authority: Pubkey,
        new_authority: Pubkey,
    },
    // Note: SystemUpgradeNonceAccount not included - no authority parameter
}

instruction_type!(ParsedSystemInstructionData => ParsedSystemInstructionType {
    SystemTransfer,
    SystemCreateAccount,
    SystemWithdrawNonceAccount,
    SystemAssign,
    SystemAllocate,
    SystemInitializeNonceAccount,
    SystemAdvanceNonceAccount,
    SystemAuthorizeNonceAccount,
});

impl IxUtils {
    pub fn parse_system_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>, KoraError>
    {
        let mut parsed_instructions: HashMap<
            ParsedSystemInstructionType,
            Vec<ParsedSystemInstructionData>,
        > = HashMap::new();

        for instruction in transaction.all_instructions.iter() {
            if instruction.program_id != SYSTEM_PROGRAM_ID {
                continue;
            }
            let key = |index: usize| instruction.accounts[index].pubkey;

            let data = match bincode::deserialize::<SystemInstruction>(&instruction.data) {
                Ok(SystemInstruction::CreateAccount { lamports, owner, .. }) => {
                    use instruction_indexes::system_create_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemCreateAccount {
                        lamports,
                        payer: key(ix::PAYER_INDEX),
                        new_account: key(ix::NEW_ACCOUNT_INDEX),
                        owner,
                        base: None,
                    }
                }
                Ok(SystemInstruction::CreateAccountWithSeed { lamports, owner, base, .. }) => {
                    use instruction_indexes::system_create_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemCreateAccount {
                        lamports,
                        payer: key(ix::PAYER_INDEX),
                        new_account: key(ix::NEW_ACCOUNT_INDEX),
                        owner,
                        base: Some(base),
                    }
                }
                Ok(SystemInstruction::Transfer { lamports }) => {
                    use instruction_indexes::system_transfer as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemTransfer {
                        lamports,
                        sender: key(ix::SENDER_INDEX),
                        receiver: key(ix::RECEIVER_INDEX),
                    }
                }
                Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                    use instruction_indexes::system_transfer_with_seed as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemTransfer {
                        lamports,
                        sender: key(ix::SENDER_INDEX),
                        receiver: key(ix::RECEIVER_INDEX),
                    }
                }
                Ok(SystemInstruction::WithdrawNonceAccount(lamports)) => {
                    use instruction_indexes::system_withdraw_nonce_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemWithdrawNonceAccount {
                        lamports,
                        nonce_authority: key(ix::NONCE_AUTHORITY_INDEX),
                        recipient: key(ix::RECIPIENT_INDEX),
                    }
                }
                Ok(SystemInstruction::Assign { owner }) => {
                    use instruction_indexes::system_assign as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAssign {
                        authority: key(ix::AUTHORITY_INDEX),
                        owner,
                    }
                }
                Ok(SystemInstruction::AssignWithSeed { owner, .. }) => {
                    use instruction_indexes::system_assign_with_seed as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAssign {
                        authority: key(ix::AUTHORITY_INDEX),
                        owner,
                    }
                }
                Ok(SystemInstruction::Allocate { .. }) => {
                    use instruction_indexes::system_allocate as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAllocate { account: key(ix::ACCOUNT_INDEX) }
                }
                Ok(SystemInstruction::AllocateWithSeed { .. }) => {
                    use instruction_indexes::system_allocate_with_seed as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAllocate { account: key(ix::ACCOUNT_INDEX) }
                }
                Ok(SystemInstruction::InitializeNonceAccount(nonce_authority)) => {
                    use instruction_indexes::system_initialize_nonce_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemInitializeNonceAccount {
                        nonce_account: key(ix::NONCE_ACCOUNT_INDEX),
                        nonce_authority,
                    }
                }
                Ok(SystemInstruction::AdvanceNonceAccount) => {
                    use instruction_indexes::system_advance_nonce_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAdvanceNonceAccount {
                        nonce_account: key(ix::NONCE_ACCOUNT_INDEX),
                        nonce_authority: key(ix::NONCE_AUTHORITY_INDEX),
                    }
                }
                Ok(SystemInstruction::AuthorizeNonceAccount(new_authority)) => {
                    use instruction_indexes::system_authorize_nonce_account as ix;
                    validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                    ParsedSystemInstructionData::SystemAuthorizeNonceAccount {
                        nonce_account: key(ix::NONCE_ACCOUNT_INDEX),
                        nonce_authority: key(ix::NONCE_AUTHORITY_INDEX),
                        new_authority,
                    }
                }
                // UpgradeNonceAccount: Not parsed - no authority parameter, cannot validate fee payer involvement
                // Anyone can upgrade any nonce account without signing
                Ok(SystemInstruction::UpgradeNonceAccount) => continue,
                _ => {
                    let Some((lamports, owner)) =
                        Self::parse_create_account_allow_prefund(&instruction.data)
                    else {
                        continue;
                    };
                    use instruction_indexes::system_create_account_allow_prefund as ix;
                    let min_accounts = if lamports > 0 {
                        ix::REQUIRED_NUMBER_OF_ACCOUNTS_WITH_FUNDING
                    } else {
                        ix::MIN_REQUIRED_NUMBER_OF_ACCOUNTS
                    };
                    validate_number_accounts!(instruction, min_accounts);
                    let new_account = key(ix::NEW_ACCOUNT_INDEX);
                    let payer = if lamports > 0 { key(ix::FUNDING_INDEX) } else { new_account };
                    ParsedSystemInstructionData::SystemCreateAccount {
                        lamports,
                        payer,
                        new_account,
                        owner,
                        base: None,
                    }
                }
            };
            parsed_instructions.entry(data.instruction_type()).or_default().push(data);
        }
        Ok(parsed_instructions)
    }

    // Absent from system-interface 2.0.0; decode (tag, lamports, space, owner) by hand.
    fn parse_create_account_allow_prefund(data: &[u8]) -> Option<(u64, Pubkey)> {
        let (tag, lamports, _space, owner) =
            bincode::deserialize::<(u32, u64, u64, Pubkey)>(data).ok()?;
        (tag == instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR)
            .then_some((lamports, owner))
    }
}
