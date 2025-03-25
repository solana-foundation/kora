use crate::{
    config::ValidationConfig, error::KoraError, get_signer,
    transaction::validator::TransactionValidator, Signer as _,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    message::VersionedMessage,
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};

pub fn decode_b58_transaction_with_version(tx: &str) -> Result<VersionedTransaction, KoraError> {
    if tx.is_empty() {
        return Err(KoraError::InvalidTransaction("Empty transaction string".to_string()));
    }

    let decoded_bytes = match bs58::decode(tx).into_vec() {
        Ok(bytes) => {
            if bytes.is_empty() {
                return Err(KoraError::InvalidTransaction("Decoded bytes are empty".to_string()));
            }
            log::debug!("Successfully decoded base58 data, length: {} bytes", bytes.len());
            bytes
        }
        Err(e) => {
            log::error!("Failed to decode base58 data: {}", e);
            return Err(KoraError::InvalidTransaction(format!("Invalid base58: {}", e)));
        }
    };

    let transaction = match bincode::deserialize::<VersionedTransaction>(&decoded_bytes) {
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
            return Err(KoraError::InvalidTransaction(format!(
                "Failed to deserialize transaction: {}",
                e
            )));
        }
    };

    Ok(transaction)
}

use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn decode_b58_transaction(tx: &str) -> Result<Transaction, KoraError> {
    if tx.is_empty() {
        return Err(KoraError::InvalidTransaction("Empty transaction string".to_string()));
    }

    let decoded_bytes = match bs58::decode(tx).into_vec() {
        Ok(bytes) => {
            if bytes.is_empty() {
                return Err(KoraError::InvalidTransaction("Decoded bytes are empty".to_string()));
            }
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
            return Err(KoraError::InvalidTransaction(format!(
                "Failed to deserialize transaction: {}",
                e
            )));
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

pub async fn sign_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(Transaction, String), KoraError> {
    let signer = get_signer()?;
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Validate transaction and accounts
    validator.validate_transaction(&transaction)?;
    validator.validate_disallowed_accounts(&transaction.message)?;

    // Get latest blockhash and update transaction
    let mut transaction = transaction;
    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;
    transaction.message.recent_blockhash = blockhash.0;

    // Validate transaction fee
    let estimated_fee = rpc_client.get_fee_for_message(&transaction.message).await?;
    validator.validate_lamport_fee(estimated_fee)?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    // Serialize signed transaction
    let serialized = bincode::serialize(&transaction)?;
    let encoded = STANDARD.encode(serialized);

    Ok((transaction, encoded))
}

pub async fn sign_and_send_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) = sign_transaction(rpc_client, validation, transaction).await?;

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    Ok((signature.to_string(), encoded))
}

pub async fn sign_versioned_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(VersionedTransaction, String), KoraError> {
    let signer = get_signer()?;
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Validate transaction and accounts
    validator.validate_transaction_with_versioned(&transaction)?;
    validator.validate_disallowed_accounts_with_versioned(transaction.message.clone())?;

    let mut transaction = transaction;

    // Get latest blockhash and update transaction
    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;
    transaction.message.set_recent_blockhash(blockhash.0);

    // Validate transaction fee
    match &transaction.message {
        VersionedMessage::V0(v0_message) => {
            // Simulate the transaction to get the compute units consumed
            let simulation_result = rpc_client
                .simulate_transaction_with_config(
                    &transaction,
                    solana_client::rpc_config::RpcSimulateTransactionConfig {
                        sig_verify: false,
                        replace_recent_blockhash: true,
                        commitment: Some(CommitmentConfig::processed()),
                        ..Default::default()
                    },
                )
                .await?;

            if let Some(units_consumed) = simulation_result.value.units_consumed {
                // Fetch the fee rate (lamports per signature)
                let blockhash_response = rpc_client
                    .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
                    .await?;
                let lamports_per_signature = blockhash_response.1;

                // Calculate the estimated fee
                let num_signatures = transaction.signatures.len() as u64;
                let num_account_keys = v0_message.account_keys.len() as u64;
                let num_lookups = v0_message.address_table_lookups.len() as u64;

                // Base fee: lamports per signature
                let base_fee = lamports_per_signature * num_signatures;

                // Additional fee: based on the number of account keys and lookups
                let additional_fee = units_consumed as u64
                + (num_account_keys * 10) // Example weight for account keys
                + (num_lookups * 20); // Example weight for lookups

                let estimated_fee = base_fee + additional_fee;

                validator.validate_lamport_fee(estimated_fee)?;
            } else {
                return Err(KoraError::InvalidTransaction(
                    "Failed to simulate transaction for fee estimation".to_string(),
                ));
            }
        }
        VersionedMessage::Legacy(legacy_message) => {
            // The existing approach for Legacy messages
            let estimated_fee = rpc_client.get_fee_for_message(legacy_message).await?;
            validator.validate_lamport_fee(estimated_fee)?;
        }
    }

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message.serialize()).await?;
    transaction.signatures[0] = signature;

    // Serialize signed transaction
    let serialized = bincode::serialize(&transaction)?;
    let encoded = bs58::encode(serialized).into_string();

    Ok((transaction, encoded))
}

pub async fn sign_and_send_transaction_with_version(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) =
        sign_versioned_transaction(rpc_client, validation, transaction).await?;

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    Ok((signature.to_string(), encoded))
}

pub fn encode_transaction_b58(transaction: &Transaction) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction).map_err(|e| {
        KoraError::SerializationError(format!("Base58 serialization failed: {}", e))
    })?;
    Ok(bs58::encode(serialized).into_string())
}

pub fn encode_transaction_b58_with_version(
    transaction: &VersionedTransaction,
) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction).map_err(|e| {
        KoraError::SerializationError(format!("Base58 serialization failed: {}", e))
    })?;
    Ok(bs58::encode(serialized).into_string())
}

pub fn encode_transaction_b64(transaction: &Transaction) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction).map_err(|e| {
        KoraError::SerializationError(format!("Base64 serialization failed: {}", e))
    })?;
    Ok(STANDARD.encode(serialized))
}

pub fn encode_transaction_b64_with_version(
    transaction: &VersionedTransaction,
) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction).map_err(|e| {
        KoraError::SerializationError(format!("Base64 serialization failed: {}", e))
    })?;
    Ok(base64::encode(serialized))
}

pub fn decode_b64_transaction(encoded: &str) -> Result<Transaction, KoraError> {
    let decoded = STANDARD.decode(encoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to decode base64 transaction: {}", e))
    })?;

    bincode::deserialize(&decoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to deserialize transaction: {}", e))
    })
}

pub fn decode_b64_transaction_with_version(
    encoded: &str,
) -> Result<VersionedTransaction, KoraError> {
    let decoded = base64::decode(encoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to decode base64 transaction: {}", e))
    })?;

    bincode::deserialize(&decoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to deserialize transaction: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{hash::Hash, message::Message, signature::Keypair, signer::Signer as _};

    #[test]
    fn test_decode_b58_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = Message::new(&[instruction], Some(&keypair.pubkey()));
        let tx = Transaction::new(&[&keypair], message, Hash::default());

        let encoded = bs58::encode(bincode::serialize(&tx).unwrap()).into_string();
        let decoded = decode_b58_transaction(&encoded).unwrap();

        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_decode_b58_transaction_invalid_input() {
        let result = decode_b58_transaction("not-base58!");
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        let result = decode_b58_transaction("3xQP"); // base58 of [1,2,3]
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }

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

        let instructions = uncompile_instructions(&[compiled_ix], &account_keys);

        assert_eq!(instructions.len(), 1);
        let uncompiled = &instructions[0];
        assert_eq!(uncompiled.program_id, program_id);
        assert_eq!(uncompiled.accounts.len(), 2);
        assert_eq!(uncompiled.accounts[0].pubkey, account1);
        assert_eq!(uncompiled.accounts[1].pubkey, account2);
        assert_eq!(uncompiled.data, vec![1, 2, 3]);
    }
}
