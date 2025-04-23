use std::future::Future;

use crate::{
    config::ValidationConfig,
    error::KoraError,
    get_signer,
    transaction::{fees::estimate_transaction_fee, validator::TransactionValidator},
    Signer as _,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};

// Optimize & consolidate transaction encoding/decoding for base64
/// Encodes any serializable data to base64 with standardized error handling
pub fn encode_b64<T: serde::Serialize>(data: &T, type_name: &str) -> Result<String, KoraError> {
    bincode::serialize(data)
        .map_err(|e| {
            KoraError::SerializationError(format!("{} serialization failed: {}", type_name, e))
        })
        .map(|bytes| STANDARD.encode(bytes))
}

/// Decodes base64-encoded data with standardized error handling
fn decode_b64<T: for<'a> serde::Deserialize<'a>>(
    encoded: &str,
    type_name: &str,
) -> Result<T, KoraError> {
    if encoded.is_empty() {
        return Err(KoraError::InvalidTransaction(format!("Empty {} string", type_name)));
    }

    STANDARD
        .decode(encoded)
        .map_err(|e| {
            KoraError::InvalidTransaction(format!("Failed to decode base64 {}: {}", type_name, e))
        })
        .and_then(|bytes| {
            bincode::deserialize(&bytes).map_err(|e| {
                log::error!(
                    "Failed to deserialize {}: {}; Decoded bytes length: {}",
                    type_name,
                    e,
                    bytes.len()
                );
                KoraError::InvalidTransaction(format!("Failed to deserialize {}: {}", type_name, e))
            })
        })
}

// Specific transaction encoding/decoding functions using the generic implementations
pub fn encode_b64_transaction(tx: &Transaction) -> Result<String, KoraError> {
    encode_b64(tx, "transaction")
}

pub fn encode_b64_transaction_with_version(tx: &VersionedTransaction) -> Result<String, KoraError> {
    encode_b64(tx, "versioned transaction")
}

pub fn encode_b64_message(message: &Message) -> Result<String, KoraError> {
    encode_b64(message, "message")
}

pub fn decode_b64_transaction(encoded: &str) -> Result<Transaction, KoraError> {
    decode_b64(encoded, "transaction")
}

pub fn decode_b64_transaction_with_version(
    encoded: &str,
) -> Result<VersionedTransaction, KoraError> {
    decode_b64(encoded, "versioned transaction")
}

/// Converts CompiledInstructions back to regular Instructions by resolving account indexes
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

/// Common transaction signing setup with consolidated error handling
async fn prepare_transaction_signing(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
) -> Result<(TransactionValidator, solana_sdk::hash::Hash), KoraError> {
    let signer = get_signer()?;
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Get latest blockhash - optimized with simpler error handling
    let (blockhash, _) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .map_err(|e| KoraError::RpcError(format!("Blockhash retrieval failed: {}", e)))?;

    Ok((validator, blockhash))
}

/// Validates that a transaction fee is within allowed limits
async fn validate_transaction_fee(
    validator: &TransactionValidator,
    fee_estimator: impl Future<Output = Result<u64, KoraError>>,
) -> Result<(), KoraError> {
    validator.validate_lamport_fee(fee_estimator.await?)
}

/// Common transaction validation logic for both transaction types
async fn validate_transaction_common<T, F>(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: &T,
    validator: &TransactionValidator,
    validate_fn: F,
    fee_estimator: impl Future<Output = Result<u64, KoraError>>,
) -> Result<(), KoraError>
where
    F: FnOnce(&TransactionValidator, &T) -> Result<(), KoraError>,
{
    // Validate transaction
    validate_fn(validator, transaction)?;

    // Validate fee
    validate_transaction_fee(validator, fee_estimator).await?;

    Ok(())
}

/// Signs a legacy Solana transaction with validation
pub async fn sign_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(Transaction, String), KoraError> {
    let signer = get_signer()?;
    let (validator, blockhash) = prepare_transaction_signing(rpc_client, validation).await?;

    // Prepare transaction with blockhash
    let mut transaction = transaction;

    // Update blockhash if needed
    if transaction.signatures.is_empty()
        || transaction.message.recent_blockhash == Default::default()
    {
        transaction.message.recent_blockhash = blockhash;
    }

    // Use validate_transaction_common for validation
    validate_transaction_common(
        rpc_client,
        validation,
        &transaction,
        &validator,
        |v, tx| {
            v.validate_transaction(tx)?;
            v.validate_disallowed_accounts(&tx.message)
        },
        estimate_transaction_fee(rpc_client, &transaction),
    )
    .await?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    // Encode transaction
    let encoded = encode_b64_transaction(&transaction)?;

    Ok((transaction, encoded))
}

/// Signs a versioned Solana transaction with validation
pub async fn sign_versioned_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(VersionedTransaction, String), KoraError> {
    let signer = get_signer()?;
    let (validator, blockhash) = prepare_transaction_signing(rpc_client, validation).await?;

    let mut transaction = transaction;
    transaction.message.set_recent_blockhash(blockhash);

    // Use validate_transaction_common for validation
    validate_transaction_common(
        rpc_client,
        validation,
        &transaction,
        &validator,
        |v, tx| {
            v.validate_transaction_with_versioned(tx)?;
            v.validate_disallowed_accounts_with_versioned(&tx.message.clone())
        },
        crate::transaction::fees::estimate_versioned_transaction_fee(rpc_client, &transaction),
    )
    .await?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message.serialize()).await?;
    transaction.signatures[0] = signature;

    // Encode transaction
    let encoded = encode_b64_transaction_with_version(&transaction)?;

    Ok((transaction, encoded))
}

/// Standardized error handling for sending transactions
async fn send_transaction_with_confirmation<T>(
    rpc_client: &RpcClient,
    transaction: &T,
    tx_type: &str,
) -> Result<String, KoraError>
where
    T: Sync + solana_client::rpc_client::SerializableTransaction,
{
    rpc_client.send_and_confirm_transaction(transaction).await.map(|sig| sig.to_string()).map_err(
        |e| {
            log::error!("{} sending failed: {}", tx_type, e);
            KoraError::RpcError(format!("{} sending failed: {}", tx_type, e))
        },
    )
}

/// Signs and sends a legacy transaction
pub async fn sign_and_send_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) = sign_transaction(rpc_client, validation, transaction).await?;
    let signature =
        send_transaction_with_confirmation(rpc_client, &transaction, "Transaction").await?;
    Ok((signature, encoded))
}

/// Signs and sends a versioned transaction
pub async fn sign_and_send_transaction_with_version(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) =
        sign_versioned_transaction(rpc_client, validation, transaction).await?;
    let signature =
        send_transaction_with_confirmation(rpc_client, &transaction, "Versioned transaction")
            .await?;
    Ok((signature, encoded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{hash::Hash, message::Message, signature::Keypair, signer::Signer as _};

    #[test]
    fn test_encode_decode_b64_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = Message::new(&[instruction], Some(&keypair.pubkey()));
        let tx = Transaction::new(&[&keypair], message, Hash::default());

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
    fn test_encode_b64_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = Message::new(&[instruction], Some(&keypair.pubkey()));
        let tx = Transaction::new(&[&keypair], message, Hash::default());

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
}
