use std::collections::HashMap;

use solana_message::compiled_instruction::CompiledInstruction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::error::KoraError;

macro_rules! instruction_type {
    ($data:ident => $ty:ident { $($variant:ident),* $(,)? }) => {
        impl $data {
            fn instruction_type(&self) -> $ty {
                match self {
                    $(Self::$variant { .. } => $ty::$variant,)*
                }
            }
        }
    };
}

macro_rules! validate_number_accounts {
    ($instruction:expr, $min_count:expr) => {
        if $instruction.accounts.len() < $min_count {
            log::error!("Instruction {:?} has less than {} accounts", $instruction, $min_count);
            return Err(KoraError::InvalidTransaction(format!(
                "Instruction doesn't have the required number of accounts",
            )));
        }
    };
}

mod alt;
mod bpf_loader_upgradeable;
mod loader_v4;
mod reconstruct;
mod spl_token;
mod system;
#[cfg(test)]
mod tests;

pub use alt::*;
pub use bpf_loader_upgradeable::*;
pub use loader_v4::*;
pub use reconstruct::*;
pub use spl_token::*;
pub use system::*;

pub struct IxUtils;

impl IxUtils {
    fn get_account_index(
        account_keys_hashmap: &HashMap<Pubkey, u8>,
        pubkey: &Pubkey,
    ) -> Result<u8, KoraError> {
        account_keys_hashmap.get(pubkey).copied().ok_or_else(|| {
            KoraError::SerializationError(format!("{} not found in account keys", pubkey))
        })
    }

    pub fn build_account_keys_hashmap(account_keys: &[Pubkey]) -> HashMap<Pubkey, u8> {
        account_keys.iter().enumerate().map(|(idx, key)| (*key, idx as u8)).collect()
    }

    pub fn get_account_key_if_present(ix: &Instruction, index: usize) -> Option<Pubkey> {
        if ix.accounts.is_empty() {
            return None;
        }

        if index >= ix.accounts.len() {
            return None;
        }

        Some(ix.accounts[index].pubkey)
    }

    pub fn get_account_key_required(
        account_keys: &[Pubkey],
        index: usize,
    ) -> Result<Pubkey, KoraError> {
        account_keys.get(index).copied().ok_or_else(|| {
            KoraError::SerializationError(format!("Account key at index {} not found", index))
        })
    }

    fn extract_multisig_signers(instruction: &Instruction, skip: usize) -> Vec<Pubkey> {
        instruction.accounts.iter().skip(skip).map(|a| a.pubkey).collect()
    }

    pub fn build_default_compiled_instruction(program_id_index: u8) -> CompiledInstruction {
        CompiledInstruction { program_id_index, accounts: vec![], data: vec![] }
    }

    pub fn uncompile_instructions(
        instructions: &[CompiledInstruction],
        account_keys: &[Pubkey],
    ) -> Result<Vec<Instruction>, KoraError> {
        instructions
            .iter()
            .map(|ix| {
                let program_id =
                    Self::get_account_key_required(account_keys, ix.program_id_index as usize)?;
                let accounts: Result<Vec<AccountMeta>, KoraError> = ix
                    .accounts
                    .iter()
                    .map(|idx| {
                        Ok(AccountMeta {
                            pubkey: Self::get_account_key_required(account_keys, *idx as usize)?,
                            is_signer: false,
                            is_writable: true,
                        })
                    })
                    .collect();

                Ok(Instruction { program_id, accounts: accounts?, data: ix.data.clone() })
            })
            .collect()
    }
}
