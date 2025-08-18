use std::collections::HashMap;

use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
    system_instruction::SystemInstruction,
    system_program::ID as SYSTEM_PROGRAM_ID,
};

use crate::transaction::VersionedTransactionResolved;

// Instruction type that we support to parse from the transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionType {
    SystemTransfer,
    SystemCreateAccount,
    SystemWithdrawNonceAccount,
    SystemAssign,
}

// Instruction type that we support to parse from the transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionData {
    // Includes transfer and transfer with seed
    SystemTransfer { lamports: u64, sender: Pubkey, receiver: Pubkey },
    // Includes create account and create account with seed
    SystemCreateAccount { lamports: u64, payer: Pubkey },
    // Includes withdraw nonce account
    SystemWithdrawNonceAccount { lamports: u64, nonce_authority: Pubkey, recipient: Pubkey },
    // Includes assign and assign with seed
    SystemAssign { authority: Pubkey },
}

pub struct IxUtils;

impl IxUtils {
    pub fn get_account_key_if_present(ix: &Instruction, index: usize) -> Option<Pubkey> {
        if ix.accounts.is_empty() {
            return None;
        }

        if index >= ix.accounts.len() {
            return None;
        }

        Some(ix.accounts[index].pubkey)
    }

    pub fn uncompile_instructions(
        instructions: &[CompiledInstruction],
        account_keys: &[Pubkey],
    ) -> Vec<Instruction> {
        instructions
            .iter()
            .map(|ix| {
                let program_id = account_keys[ix.program_id_index as usize];
                let accounts = ix
                    .accounts
                    .iter()
                    .map(|idx| AccountMeta {
                        pubkey: account_keys[*idx as usize],
                        is_signer: false,
                        is_writable: true,
                    })
                    .collect();

                Instruction { program_id, accounts, data: ix.data.clone() }
            })
            .collect()
    }

    pub fn parse_system_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>> {
        let mut parsed_instructions: HashMap<
            ParsedSystemInstructionType,
            Vec<ParsedSystemInstructionData>,
        > = HashMap::new();

        for instruction in &transaction.all_instructions {
            let program_id = instruction.program_id;

            // Handle System Program transfers and account creation
            if program_id == SYSTEM_PROGRAM_ID {
                match bincode::deserialize::<SystemInstruction>(&instruction.data) {
                    // Account creation instructions - funding account pays lamports
                    Ok(SystemInstruction::CreateAccount { lamports, .. })
                    | Ok(SystemInstruction::CreateAccountWithSeed { lamports, .. }) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemCreateAccount)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemCreateAccount {
                                lamports,
                                payer: instruction.accounts[0].pubkey,
                            });
                    }
                    // Transfer instructions
                    Ok(SystemInstruction::Transfer { lamports }) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemTransfer)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemTransfer {
                                lamports,
                                sender: instruction.accounts[0].pubkey,
                                receiver: instruction.accounts[1].pubkey,
                            });
                    }
                    Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemTransfer)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemTransfer {
                                lamports,
                                sender: instruction.accounts[1].pubkey,
                                receiver: instruction.accounts[2].pubkey,
                            });
                    }
                    // Nonce account withdrawal
                    Ok(SystemInstruction::WithdrawNonceAccount(lamports)) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemWithdrawNonceAccount)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemWithdrawNonceAccount {
                                lamports,
                                nonce_authority: instruction.accounts[4].pubkey,
                                recipient: instruction.accounts[1].pubkey,
                            });
                    }
                    Ok(SystemInstruction::Assign { .. }) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemAssign)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemAssign {
                                authority: instruction.accounts[0].pubkey,
                            });
                    }
                    Ok(SystemInstruction::AssignWithSeed { .. }) => {
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemAssign)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemAssign {
                                authority: instruction.accounts[1].pubkey,
                            });
                    }
                    _ => {}
                }
            }
        }
        parsed_instructions
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_uncompile_instructions() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let account_keys = vec![program_id, account1, account2];
        let compiled_ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![1, 2], // indices into account_keys
            data: vec![1, 2, 3],
        };

        let instructions = IxUtils::uncompile_instructions(&[compiled_ix], &account_keys);

        assert_eq!(instructions.len(), 1);
        let uncompiled = &instructions[0];
        assert_eq!(uncompiled.program_id, program_id);
        assert_eq!(uncompiled.accounts.len(), 2);
        assert_eq!(uncompiled.accounts[0].pubkey, account1);
        assert_eq!(uncompiled.accounts[1].pubkey, account2);
        assert_eq!(uncompiled.data, vec![1, 2, 3]);
    }
}
