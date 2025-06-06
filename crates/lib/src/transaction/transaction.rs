use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
};

use crate::{
    config::ValidationConfig, error::KoraError, get_signer,
    transaction::validator::TransactionValidator, Signer as _,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};

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
    if transaction.signatures.is_empty() {
        let blockhash =
            rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;
        transaction.message.recent_blockhash = blockhash.0;
    }

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

pub fn encode_b64_transaction(transaction: &Transaction) -> Result<String, KoraError> {
    let serialized = bincode::serialize(transaction).map_err(|e| {
        KoraError::SerializationError(format!("Base64 serialization failed: {}", e))
    })?;
    Ok(STANDARD.encode(serialized))
}

pub fn encode_b64_message(message: &Message) -> Result<String, KoraError> {
    let serialized = bincode::serialize(message).map_err(|e| {
        KoraError::SerializationError(format!("Base64 serialization failed: {}", e))
    })?;
    Ok(STANDARD.encode(serialized))
}

pub fn decode_b64_transaction(encoded: &str) -> Result<Transaction, KoraError> {
    let decoded = STANDARD.decode(encoded).map_err(|e| {
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
    fn test_encode_transaction_b64() {
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
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use anyhow::Result;

pub fn create_transfer_transaction(
    rpc_client: &RpcClient,
    from_keypair: &Keypair,
    to_pubkey: &Pubkey,
    lamports: u64,
    recent_blockhash: Option<solana_sdk::hash::Hash>,
) -> Result<Transaction> {
    let blockhash = match recent_blockhash {
        Some(hash) => hash,
        None => rpc_client.get_latest_blockhash()?,
    };

    let instruction: Instruction = system_instruction::transfer(
        &from_keypair.pubkey(),
        to_pubkey,
        lamports,
    );

    let message = Message::new(&[instruction], Some(&from_keypair.pubkey()));

    let tx = Transaction::new(&[from_keypair], message, blockhash);

    Ok(tx)
}

