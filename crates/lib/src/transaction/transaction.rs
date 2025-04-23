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
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};

/// Decodes a base58-encoded string into bytes with comprehensive error handling.
fn decode_base58(tx: &str) -> Result<Vec<u8>, KoraError> {
    if tx.is_empty() {
        return Err(KoraError::InvalidTransaction("Empty transaction string".to_string()));
    }

    bs58::decode(tx)
        .into_vec()
        .map_err(|e| {
            log::error!("Failed to decode base58 data: {}", e);
            KoraError::InvalidTransaction(format!("Invalid base58: {}", e))
        })
        .and_then(|bytes| {
            if bytes.is_empty() {
                Err(KoraError::InvalidTransaction("Decoded bytes are empty".to_string()))
            } else {
                log::debug!("Successfully decoded base58 data: {} bytes", bytes.len());
                Ok(bytes)
            }
        })
}

/// Decodes a base64-encoded string into bytes with error handling.
fn decode_base64(encoded: &str, type_name: &str) -> Result<Vec<u8>, KoraError> {
    if encoded.is_empty() {
        return Err(KoraError::InvalidTransaction(format!("Empty {} string", type_name)));
    }

    STANDARD.decode(encoded).map_err(|e| {
        log::error!("Failed to decode base64 {}: {}", type_name, e);
        KoraError::InvalidTransaction(format!("Failed to decode base64 {}: {}", type_name, e))
    })
}

/// Generic function to deserialize transaction data with detailed error handling.
fn deserialize_transaction<T: for<'a> serde::Deserialize<'a>>(
    bytes: &[u8],
    type_name: &str,
) -> Result<T, KoraError> {
    bincode::deserialize(bytes).map_err(|e| {
        log::error!(
            "Failed to deserialize {}: {}; Decoded bytes length: {}",
            type_name,
            e,
            bytes.len()
        );
        KoraError::InvalidTransaction(format!("Failed to deserialize {}: {}", type_name, e))
    })
}

/// Generic function for transaction encoding with unified error handling.
fn encode_transaction<T: serde::Serialize>(
    transaction: &T,
    encoder: impl FnOnce(&[u8]) -> String,
) -> Result<String, KoraError> {
    bincode::serialize(transaction)
        .map_err(|e| KoraError::SerializationError(format!("Serialization failed: {}", e)))
        .map(|serialized| encoder(&serialized))
}

// Improved macro for creating encoding functions with consistent patterns
macro_rules! define_encoding_functions {
    ($(($name:ident, $type:ty, $encoder:expr)),*) => {
        $(
            pub fn $name(transaction: &$type) -> Result<String, KoraError> {
                encode_transaction(transaction, $encoder)
            }
        )*
    };
}

// Improved macro for creating decoding functions with consistent patterns
macro_rules! define_decoding_functions {
    ($(($name:ident, $type:ty, $type_name:expr, $decode_fn:expr)),*) => {
        $(
            pub fn $name(encoded: &str) -> Result<$type, KoraError> {
                let decoded = $decode_fn(encoded)?;
                deserialize_transaction::<$type>(&decoded, $type_name)
            }
        )*
    };
}

// Define all encoding functions using the macro
define_encoding_functions!(
    (encode_transaction_b58, Transaction, |serialized| bs58::encode(serialized).into_string()),
    (encode_transaction_b58_with_version, VersionedTransaction, |serialized| bs58::encode(
        serialized
    )
    .into_string()),
    (encode_transaction_b64, Transaction, |serialized| STANDARD.encode(serialized)),
    (encode_transaction_b64_with_version, VersionedTransaction, |serialized| STANDARD
        .encode(serialized))
);

// Define all decoding functions using the macro
define_decoding_functions!(
    (decode_b58_transaction, Transaction, "transaction", decode_base58),
    (
        decode_b58_transaction_with_version,
        VersionedTransaction,
        "versioned transaction",
        decode_base58
    ),
    (decode_b64_transaction, Transaction, "transaction", |encoded| decode_base64(
        encoded,
        "transaction"
    )),
    (
        decode_b64_transaction_with_version,
        VersionedTransaction,
        "versioned transaction",
        |encoded| decode_base64(encoded, "versioned transaction")
    )
);

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

    // Get latest blockhash
    let (blockhash, _) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .map_err(|e| {
            log::error!("Failed to get latest blockhash: {}", e);
            KoraError::RpcError(format!("Blockhash retrieval failed: {}", e))
        })?;

    Ok((validator, blockhash))
}

/// Validates that a transaction fee is within allowed limits
async fn validate_transaction_fee(
    validator: &TransactionValidator,
    fee_estimator: impl Future<Output = Result<u64, KoraError>>,
) -> Result<(), KoraError> {
    let fee = fee_estimator.await?;
    validator.validate_lamport_fee(fee)
}

/// Signs a legacy Solana transaction with validation
pub async fn sign_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(Transaction, String), KoraError> {
    let signer = get_signer()?;
    let (validator, blockhash) = prepare_transaction_signing(rpc_client, validation).await?;

    // Validate transaction and accounts
    validator.validate_transaction(&transaction)?;
    validator.validate_disallowed_accounts(&transaction.message)?;

    // Update blockhash
    let mut transaction = transaction;
    transaction.message.recent_blockhash = blockhash;

    // Validate transaction fee - with updated function call
    validate_transaction_fee(&validator, estimate_transaction_fee(rpc_client, &transaction))
        .await?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    // Encode transaction
    let encoded = encode_transaction_b64(&transaction)?;

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

    // Validate transaction and accounts
    validator.validate_transaction_with_versioned(&transaction)?;
    validator.validate_disallowed_accounts_with_versioned(transaction.message.clone())?;

    let mut transaction = transaction;
    transaction.message.set_recent_blockhash(blockhash);

    // Validate transaction fee - with updated function call
    validate_transaction_fee(
        &validator,
        crate::transaction::fees::estimate_versioned_transaction_fee(rpc_client, &transaction),
    )
    .await?;

    // Sign transaction
    let signature = signer.sign_solana(&transaction.message.serialize()).await?;
    transaction.signatures[0] = signature;

    // Encode transaction
    let encoded = encode_transaction_b58_with_version(&transaction)?;

    Ok((transaction, encoded))
}

/// Signs and sends a legacy transaction
pub async fn sign_and_send_transaction(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) = sign_transaction(rpc_client, validation, transaction).await?;

    // Send and confirm transaction
    let signature = rpc_client.send_and_confirm_transaction(&transaction).await.map_err(|e| {
        log::error!("Transaction sending failed: {}", e);
        KoraError::RpcError(format!("Transaction sending failed: {}", e))
    })?;

    Ok((signature.to_string(), encoded))
}

/// Signs and sends a versioned transaction
pub async fn sign_and_send_transaction_with_version(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: VersionedTransaction,
) -> Result<(String, String), KoraError> {
    let (transaction, encoded) =
        sign_versioned_transaction(rpc_client, validation, transaction).await?;

    // Send and confirm transaction
    let signature = rpc_client.send_and_confirm_transaction(&transaction).await.map_err(|e| {
        log::error!("Versioned transaction sending failed: {}", e);
        KoraError::RpcError(format!("Versioned transaction sending failed: {}", e))
    })?;

    Ok((signature.to_string(), encoded))
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
