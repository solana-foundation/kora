//! Jito tip instruction utilities
//!
//! Functions for creating, detecting, and managing Jito tip instructions.

use crate::{
    constant::JITO_TIP_ACCOUNTS,
    error::KoraError,
    jito::{is_tip_account, types::TipInfo},
};
use rand::Rng;
use solana_message::{compiled_instruction::CompiledInstruction, v0::Message as V0Message};
use solana_sdk::{
    instruction::Instruction, message::VersionedMessage, pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use solana_system_interface::{instruction::transfer, program::ID as SYSTEM_PROGRAM_ID};
use std::str::FromStr;

/// Selects a random tip account from the 8 available Jito tip accounts
pub fn get_random_tip_account() -> Pubkey {
    let mut rng = rand::rng();
    let index = rng.random_range(0..JITO_TIP_ACCOUNTS.len());
    Pubkey::from_str(JITO_TIP_ACCOUNTS[index])
        .expect("Invalid hardcoded tip account - this should never happen")
}

/// Creates a SOL transfer instruction to a Jito tip account
pub fn create_tip_instruction(from: &Pubkey, tip_lamports: u64) -> Instruction {
    let tip_account = get_random_tip_account();
    transfer(from, &tip_account, tip_lamports)
}

/// Creates a tip instruction with a specific tip account
pub fn create_tip_instruction_to_account(
    from: &Pubkey,
    tip_account: &Pubkey,
    tip_lamports: u64,
) -> Instruction {
    transfer(from, tip_account, tip_lamports)
}

/// Finds an existing tip instruction in a transaction
pub fn find_tip_in_transaction(transaction: &VersionedTransaction) -> Option<TipInfo> {
    let message = &transaction.message;
    let account_keys = message.static_account_keys();

    for (ix_index, instruction) in message.instructions().iter().enumerate() {
        // Check if this is a System Program transfer instruction
        let program_id = &account_keys[instruction.program_id_index as usize];
        if *program_id != SYSTEM_PROGRAM_ID {
            continue;
        }

        // System transfer instruction has instruction type 2 and minimum 2 accounts
        if instruction.data.len() < 4 || instruction.accounts.len() < 2 {
            continue;
        }

        // Check if it's a transfer instruction (instruction type 2)
        let instruction_type = u32::from_le_bytes([
            instruction.data[0],
            instruction.data[1],
            instruction.data[2],
            instruction.data[3],
        ]);
        if instruction_type != 2 {
            continue;
        }

        // Get the destination account
        let dest_account_index = instruction.accounts[1] as usize;
        if dest_account_index >= account_keys.len() {
            continue;
        }
        let dest_account = &account_keys[dest_account_index];

        // Check if destination is a Jito tip account
        if is_tip_account(dest_account) {
            // Extract the transfer amount (lamports are at bytes 4-12)
            if instruction.data.len() >= 12 {
                let amount = u64::from_le_bytes([
                    instruction.data[4],
                    instruction.data[5],
                    instruction.data[6],
                    instruction.data[7],
                    instruction.data[8],
                    instruction.data[9],
                    instruction.data[10],
                    instruction.data[11],
                ]);

                return Some(TipInfo {
                    transaction_index: 0, // Will be set by caller for bundles
                    instruction_index: ix_index,
                    amount_lamports: amount,
                    tip_account: *dest_account,
                });
            }
        }
    }

    None
}

/// Finds a tip instruction across all transactions in a bundle
pub fn find_tip_in_bundle(transactions: &[VersionedTransaction]) -> Option<TipInfo> {
    for (tx_index, tx) in transactions.iter().enumerate() {
        if let Some(mut tip_info) = find_tip_in_transaction(tx) {
            tip_info.transaction_index = tx_index;
            return Some(tip_info);
        }
    }
    None
}

/// Adds a tip instruction to the last transaction in a bundle
///
/// This modifies the transaction by adding a System transfer instruction
/// to a randomly selected Jito tip account.
pub fn add_tip_to_transaction(
    transaction: &mut VersionedTransaction,
    tip_lamports: u64,
    fee_payer: &Pubkey,
) -> Result<(), KoraError> {
    let tip_account = get_random_tip_account();

    match &mut transaction.message {
        VersionedMessage::Legacy(message) => {
            // Add tip account to account keys if not present
            let tip_account_index =
                if let Some(idx) = message.account_keys.iter().position(|k| k == &tip_account) {
                    idx as u8
                } else {
                    message.account_keys.push(tip_account);
                    (message.account_keys.len() - 1) as u8
                };

            // Find fee payer index
            let fee_payer_index =
                message.account_keys.iter().position(|k| k == fee_payer).ok_or_else(|| {
                    KoraError::InvalidTransaction("Fee payer not found in transaction".to_string())
                })? as u8;

            // Find system program index or add it
            let system_program_index = if let Some(idx) =
                message.account_keys.iter().position(|k| k == &SYSTEM_PROGRAM_ID)
            {
                idx as u8
            } else {
                message.account_keys.push(SYSTEM_PROGRAM_ID);
                (message.account_keys.len() - 1) as u8
            };

            // Create transfer instruction data
            let mut data = vec![2, 0, 0, 0]; // Transfer instruction type
            data.extend_from_slice(&tip_lamports.to_le_bytes());

            // Add compiled instruction
            message.instructions.push(CompiledInstruction {
                program_id_index: system_program_index,
                accounts: vec![fee_payer_index, tip_account_index],
                data,
            });

            // Reset signatures since message changed
            transaction.signatures =
                vec![Default::default(); message.header.num_required_signatures as usize];
        }
        VersionedMessage::V0(message) => {
            // For V0 messages, we need to rebuild the message
            let mut account_keys = message.account_keys.clone();
            let mut instructions = message.instructions.clone();

            // Add tip account if not present
            let tip_account_index =
                if let Some(idx) = account_keys.iter().position(|k| k == &tip_account) {
                    idx as u8
                } else {
                    account_keys.push(tip_account);
                    (account_keys.len() - 1) as u8
                };

            // Find fee payer index
            let fee_payer_index =
                account_keys.iter().position(|k| k == fee_payer).ok_or_else(|| {
                    KoraError::InvalidTransaction("Fee payer not found in transaction".to_string())
                })? as u8;

            // Find or add system program
            let system_program_index =
                if let Some(idx) = account_keys.iter().position(|k| k == &SYSTEM_PROGRAM_ID) {
                    idx as u8
                } else {
                    account_keys.push(SYSTEM_PROGRAM_ID);
                    (account_keys.len() - 1) as u8
                };

            // Create transfer instruction data
            let mut data = vec![2, 0, 0, 0]; // Transfer instruction type
            data.extend_from_slice(&tip_lamports.to_le_bytes());

            // Add instruction
            instructions.push(CompiledInstruction {
                program_id_index: system_program_index,
                accounts: vec![fee_payer_index, tip_account_index],
                data,
            });

            // Rebuild V0 message
            let new_message = V0Message {
                header: message.header,
                account_keys,
                recent_blockhash: message.recent_blockhash,
                instructions,
                address_table_lookups: message.address_table_lookups.clone(),
            };

            transaction.message = VersionedMessage::V0(new_message);

            // Reset signatures
            transaction.signatures = vec![
                Default::default();
                transaction.message.header().num_required_signatures
                    as usize
            ];
        }
    }

    Ok(())
}

/// Validates that a tip amount meets the minimum requirement
pub fn validate_tip_amount(tip_lamports: u64, min_tip_lamports: u64) -> Result<(), KoraError> {
    if tip_lamports < min_tip_lamports {
        return Err(KoraError::ValidationError(format!(
            "Tip amount {} lamports is below minimum required {} lamports",
            tip_lamports, min_tip_lamports
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_message::Message;
    use solana_sdk::{hash::Hash, message::VersionedMessage, signature::Keypair, signer::Signer};

    #[test]
    fn test_get_random_tip_account() {
        let account1 = get_random_tip_account();
        // Should return a valid pubkey
        assert!(is_tip_account(&account1));
    }

    #[test]
    fn test_create_tip_instruction() {
        let from = Keypair::new().pubkey();
        let instruction = create_tip_instruction(&from, 10000);

        assert_eq!(instruction.program_id, SYSTEM_PROGRAM_ID);
        assert_eq!(instruction.accounts.len(), 2);
        assert_eq!(instruction.accounts[0].pubkey, from);
        assert!(is_tip_account(&instruction.accounts[1].pubkey));
    }

    #[test]
    fn test_find_tip_in_transaction() {
        let payer = Keypair::new();
        let tip_account = Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap();
        let tip_ix = transfer(&payer.pubkey(), &tip_account, 10000);

        let message = Message::new(&[tip_ix], Some(&payer.pubkey()));
        let tx = VersionedTransaction {
            signatures: vec![Default::default()],
            message: VersionedMessage::Legacy(message),
        };

        let tip_info = find_tip_in_transaction(&tx).expect("Should find tip");
        assert_eq!(tip_info.amount_lamports, 10000);
        assert_eq!(tip_info.tip_account, tip_account);
    }

    #[test]
    fn test_find_tip_in_transaction_no_tip() {
        let payer = Keypair::new();
        let other_account = Keypair::new().pubkey();
        let transfer_ix = transfer(&payer.pubkey(), &other_account, 10000);

        let message = Message::new(&[transfer_ix], Some(&payer.pubkey()));
        let tx = VersionedTransaction {
            signatures: vec![Default::default()],
            message: VersionedMessage::Legacy(message),
        };

        assert!(find_tip_in_transaction(&tx).is_none());
    }

    #[test]
    fn test_add_tip_to_legacy_transaction() {
        let payer = Keypair::new();
        let other_account = Keypair::new().pubkey();
        let transfer_ix = transfer(&payer.pubkey(), &other_account, 1000);

        let message =
            Message::new_with_blockhash(&[transfer_ix], Some(&payer.pubkey()), &Hash::default());
        let mut tx = VersionedTransaction {
            signatures: vec![Default::default()],
            message: VersionedMessage::Legacy(message),
        };

        // Add tip
        add_tip_to_transaction(&mut tx, 10000, &payer.pubkey()).unwrap();

        // Verify tip was added
        let tip_info = find_tip_in_transaction(&tx).expect("Should find tip after adding");
        assert_eq!(tip_info.amount_lamports, 10000);
    }

    #[test]
    fn test_validate_tip_amount() {
        assert!(validate_tip_amount(10000, 1000).is_ok());
        assert!(validate_tip_amount(1000, 1000).is_ok());
        assert!(validate_tip_amount(999, 1000).is_err());
    }
}
