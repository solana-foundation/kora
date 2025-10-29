use solana_message::VersionedMessage;
use solana_sdk::{
    signature::Signature,
    transaction::{Transaction, VersionedTransaction},
};

use crate::{error::KoraError, transaction::VersionedTransactionResolved};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub struct TransactionUtil {}

impl TransactionUtil {
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

    pub fn new_unsigned_versioned_transaction(message: VersionedMessage) -> VersionedTransaction {
        let num_required_signatures = message.header().num_required_signatures as usize;
        VersionedTransaction {
            signatures: vec![Signature::default(); num_required_signatures],
            message,
        }
    }

    pub fn new_unsigned_versioned_transaction_resolved(
        message: VersionedMessage,
    ) -> Result<VersionedTransactionResolved, KoraError> {
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        VersionedTransactionResolved::from_kora_built_transaction(&transaction)
    }

    pub fn encode_versioned_transaction(
        transaction: &VersionedTransaction,
    ) -> Result<String, KoraError> {
        let serialized = bincode::serialize(transaction).map_err(|_| {
            KoraError::SerializationError("Failed to serialize transaction.".to_string())
        })?;
        Ok(STANDARD.encode(serialized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::KoraError;
    use solana_message::{compiled_instruction::CompiledInstruction, v0, Message};
    use solana_sdk::{
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer as _,
    };

    #[test]
    fn test_decode_b64_transaction_invalid_input() {
        let result = TransactionUtil::decode_b64_transaction("not-base64!");
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        let result = TransactionUtil::decode_b64_transaction("AQID"); // base64 of [1,2,3]
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
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

        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message.clone());

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

        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message.clone());

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

        let decoded = TransactionUtil::decode_b64_transaction(&encoded).unwrap();

        match decoded.message {
            VersionedMessage::Legacy(msg) => {
                assert_eq!(msg.instructions.len(), 1);
                assert_eq!(msg.account_keys.len(), 2); // keypair + program_id
            }
            VersionedMessage::V0(_) => panic!("Expected legacy message after conversion"),
        }
    }
}
