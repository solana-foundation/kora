use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_message::{v0::MessageAddressTableLookup, VersionedMessage};
use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
    signature::Signature,
    transaction::{Transaction, VersionedTransaction},
};

use crate::{
    config::ValidationConfig, error::KoraError, get_signer,
    transaction::validator::TransactionValidator, Signer as _,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_address_lookup_table_interface::state::AddressLookupTable;

pub fn new_unsigned_versioned_transaction(message: VersionedMessage) -> VersionedTransaction {
    let num_required_signatures = message.header().num_required_signatures as usize;
    VersionedTransaction {
        signatures: vec![Signature::default(); num_required_signatures],
        message,
    }
}

pub async fn get_estimate_fee(
    rpc_client: &RpcClient,
    message: &VersionedMessage,
) -> Result<u64, KoraError> {
    match message {
        VersionedMessage::Legacy(message) => rpc_client.get_fee_for_message(message).await,
        VersionedMessage::V0(message) => rpc_client.get_fee_for_message(message).await,
    }
    .map_err(|e| KoraError::RpcError(e.to_string()))
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
    transaction: VersionedTransaction,
) -> Result<(VersionedTransaction, String), KoraError> {
    let signer = get_signer()?;
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Validate transaction and accounts with lookup table resolution for V0 transactions
    validator.validate_transaction(&transaction, Some(rpc_client)).await?;

    // Get latest blockhash and update transaction
    let mut transaction = transaction;
    if transaction.signatures.is_empty() {
        let blockhash =
            rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;
        transaction.message.set_recent_blockhash(blockhash.0);
    }

    // Validate transaction fee
    let estimated_fee = get_estimate_fee(rpc_client, &transaction.message).await?;
    validator.validate_lamport_fee(estimated_fee)?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction).await?;
    transaction.signatures[0] = signature;

    // Serialize signed transaction
    let serialized = bincode::serialize(&transaction)?;
    let encoded = STANDARD.encode(serialized);

    Ok((transaction, encoded))
}

pub async fn sign_and_send_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) = sign_transaction(rpc_client, validation, transaction).await?;

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    Ok((signature.to_string(), encoded))
}

pub fn encode_b64_transaction(transaction: &VersionedTransaction) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction)
        .map_err(|e| KoraError::SerializationError(format!("Base64 serialization failed: {e}")))?;
    Ok(STANDARD.encode(serialized))
}

pub fn encode_b64_message(message: &VersionedMessage) -> Result<String, KoraError> {
    let serialized = message.serialize();
    Ok(STANDARD.encode(serialized))
}

pub fn decode_b64_transaction(encoded: &str) -> Result<VersionedTransaction, KoraError> {
    let decoded = STANDARD.decode(encoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to decode base64 transaction: {e}"))
    })?;

    // First try to deserialize as VersionedTransaction
    if let Ok(versioned_tx) = bincode::deserialize::<VersionedTransaction>(&decoded) {
        return Ok(versioned_tx);
    }

    // Fall back to legacy Transaction and convert to VersionedTransaction
    let legacy_tx: Transaction = bincode::deserialize(&decoded).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to deserialize transaction: {e}"))
    })?;

    // Convert legacy Transaction to VersionedTransaction
    Ok(VersionedTransaction {
        signatures: legacy_tx.signatures,
        message: VersionedMessage::Legacy(legacy_tx.message),
    })
}

/// Resolves addresses from lookup tables for V0 transactions
pub async fn resolve_lookup_table_addresses(
    rpc_client: &RpcClient,
    lookup_table_lookups: &[MessageAddressTableLookup],
) -> Result<Vec<Pubkey>, KoraError> {
    let mut resolved_addresses = Vec::new();

    // Maybe we can use caching here, there's a chance the lookup tables get updated though, so tbd
    for lookup in lookup_table_lookups {
        let lookup_table_account = rpc_client
            .get_account(&lookup.account_key)
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to fetch lookup table: {e}")))?;

        // Parse the lookup table account data to get the actual addresses
        let address_lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data)
            .map_err(|e| {
                KoraError::InvalidTransaction(format!("Failed to deserialize lookup table: {e}"))
            })?;

        // Resolve writable addresses
        for &index in &lookup.writable_indexes {
            if let Some(address) = address_lookup_table.addresses.get(index as usize) {
                resolved_addresses.push(*address);
            } else {
                return Err(KoraError::InvalidTransaction(format!(
                    "Lookup table index {index} out of bounds for writable addresses"
                )));
            }
        }

        // Resolve readonly addresses
        for &index in &lookup.readonly_indexes {
            if let Some(address) = address_lookup_table.addresses.get(index as usize) {
                resolved_addresses.push(*address);
            } else {
                return Err(KoraError::InvalidTransaction(format!(
                    "Lookup table index {index} out of bounds for readonly addresses"
                )));
            }
        }
    }

    Ok(resolved_addresses)
}

pub fn find_signer_position(
    transaction: &VersionedTransaction,
    signer_pubkey: &Pubkey,
) -> Result<usize, KoraError> {
    transaction
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == signer_pubkey)
        .ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Signer {signer_pubkey} not found in transaction account keys"
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::*;
    use crate::error::KoraError;
    use serde_json::json;
    use solana_address_lookup_table_interface::state::LookupTableMeta;
    use solana_client::rpc_request::RpcRequest;
    use solana_message::{v0, Message};
    use solana_sdk::{
        account::Account, hash::Hash, signature::Keypair, signer::Signer as _,
        transaction::VersionedTransaction,
    };

    fn get_mock_rpc_client(account: &Account) -> Arc<RpcClient> {
        let mut mocks = HashMap::new();
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&account.data);
        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": {
                    "slot": 1
                },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                    "rentEpoch": account.rent_epoch
                }
            }),
        );
        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    #[test]
    fn test_encode_decode_b64_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let encoded = encode_b64_transaction(&tx).unwrap();
        let decoded = decode_b64_transaction(&encoded).unwrap();

        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_decode_b64_transaction_invalid_input() {
        let result = decode_b64_transaction("not-base64!");
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        let result = decode_b64_transaction("AQID"); // base64 of [1,2,3]
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }

    #[test]
    fn test_encode_transaction_b64() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let encoded = encode_b64_transaction(&tx).unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
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

    #[test]
    fn test_new_unsigned_versioned_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));

        let transaction = new_unsigned_versioned_transaction(message.clone());

        // Should have correct number of signatures (all default/empty)
        assert_eq!(transaction.signatures.len(), message.header().num_required_signatures as usize);
        // All signatures should be default (empty)
        for sig in &transaction.signatures {
            assert_eq!(*sig, Signature::default());
        }
        assert_eq!(transaction.message, message);
    }

    #[test]
    fn test_new_unsigned_versioned_transaction_v0() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        // Create V0 message
        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![keypair.pubkey(), instruction.program_id],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: instruction.data,
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);

        let transaction = new_unsigned_versioned_transaction(message.clone());

        assert_eq!(transaction.signatures.len(), 1);
        assert_eq!(transaction.signatures[0], Signature::default());
        assert_eq!(transaction.message, message);
    }

    #[test]
    fn test_decode_b64_transaction_legacy_fallback() {
        // Test that we can decode legacy transactions and convert them to versioned
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let legacy_message = Message::new(&[instruction], Some(&keypair.pubkey()));
        let legacy_tx = Transaction::new(&[&keypair], legacy_message, Hash::default());

        let serialized = bincode::serialize(&legacy_tx).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(serialized);

        let decoded = decode_b64_transaction(&encoded).unwrap();

        match decoded.message {
            VersionedMessage::Legacy(msg) => {
                assert_eq!(msg.instructions.len(), 1);
                assert_eq!(msg.account_keys.len(), 2); // keypair + program_id
            }
            VersionedMessage::V0(_) => panic!("Expected legacy message after conversion"),
        }
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses() {
        let lookup_account_key = Pubkey::new_unique();
        let address1 = Pubkey::new_unique();
        let address2 = Pubkey::new_unique();
        let address3 = Pubkey::new_unique();

        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: u64::MAX,
                last_extended_slot: 0,
                last_extended_slot_start_index: 0,
                authority: Some(Pubkey::new_unique()),
                _padding: 0,
            },
            addresses: vec![address1, address2, address3].into(),
        };

        let serialized_data = lookup_table.serialize_for_tests().unwrap();

        let rpc_client = get_mock_rpc_client(&Account {
            data: serialized_data,
            executable: false,
            lamports: 0,
            owner: Pubkey::new_unique(),
            rent_epoch: 0,
        });

        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: lookup_account_key,
            writable_indexes: vec![0, 2], // address1, address3
            readonly_indexes: vec![1],    // address2
        }];

        let resolved_addresses =
            resolve_lookup_table_addresses(&rpc_client, &lookups).await.unwrap();

        assert_eq!(resolved_addresses.len(), 3);
        assert_eq!(resolved_addresses[0], address1);
        assert_eq!(resolved_addresses[1], address3);
        assert_eq!(resolved_addresses[2], address2);
    }

    #[test]
    fn test_find_signer_position_success() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = new_unsigned_versioned_transaction(message);

        let position = find_signer_position(&transaction, &keypair.pubkey()).unwrap();
        assert_eq!(position, 0); // Fee payer is typically at position 0
    }

    #[test]
    fn test_find_signer_position_success_v0() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let other_account = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![keypair.pubkey(), other_account, program_id],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2,
                accounts: vec![0, 1],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);

        let position = find_signer_position(&transaction, &keypair.pubkey()).unwrap();
        assert_eq!(position, 0);

        let other_position = find_signer_position(&transaction, &other_account).unwrap();
        assert_eq!(other_position, 1);
    }

    #[test]
    fn test_find_signer_position_not_found() {
        let keypair = Keypair::new();
        let missing_keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = new_unsigned_versioned_transaction(message);

        let result = find_signer_position(&transaction, &missing_keypair.pubkey());
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains(&missing_keypair.pubkey().to_string()));
            assert!(msg.contains("not found in transaction account keys"));
        }
    }

    #[test]
    fn test_find_signer_position_empty_account_keys() {
        // Create a transaction with minimal account keys
        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![], // Empty account keys
            recent_blockhash: Hash::default(),
            instructions: vec![],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);
        let search_key = Pubkey::new_unique();

        let result = find_signer_position(&transaction, &search_key);
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }
}
