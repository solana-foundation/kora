use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
    transaction::Transaction,
};

use crate::error::KoraError;

pub fn decode_b58_transaction(tx: &str) -> Result<Transaction, KoraError> {
    let decoded_bytes = match bs58::decode(tx).into_vec() {
        Ok(bytes) => {
            log::debug!("Successfully decoded base58 data, length: {} bytes", bytes.len());
            bytes
        }
        Err(e) => {
            log::error!("Failed to decode base58 data: {}", e);
            return Err(KoraError::InvalidTransaction(format!("Invalid base58: {}", e)));
        }
    };

    let transaction = match bincode::deserialize::<Transaction>(&decoded_bytes) {
        Ok(tx) => {
            log::debug!("Successfully deserialized transaction");
            tx
        }
        Err(e) => {
            log::error!(
                "Failed to deserialize transaction: {}; Decoded bytes length: {}",
                e,
                decoded_bytes.len()
            );
            return Err(KoraError::InvalidTransaction(format!("Invalid transaction: {}", e)));
        }
    };

    Ok(transaction)
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
