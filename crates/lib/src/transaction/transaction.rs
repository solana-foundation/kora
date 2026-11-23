use solana_message::{v1, VersionedMessage};
use solana_sdk::{signature::Signature, transaction::VersionedTransaction};

use crate::{error::KoraError, transaction::VersionedTransactionResolved};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub struct TransactionUtil {}

impl TransactionUtil {
    pub fn decode_b64_transaction(encoded: &str) -> Result<VersionedTransaction, KoraError> {
        let decoded = STANDARD.decode(encoded).map_err(|e| {
            KoraError::InvalidTransaction(format!("Failed to decode base64 transaction: {e}"))
        })?;

        if decoded.len() > v1::MAX_TRANSACTION_SIZE {
            return Err(KoraError::InvalidTransaction(format!(
                "Transaction size {} exceeds maximum {}",
                decoded.len(),
                v1::MAX_TRANSACTION_SIZE
            )));
        }

        let transaction =
            wincode::deserialize_exact::<VersionedTransaction>(&decoded).map_err(|e| {
                KoraError::InvalidTransaction(format!("Failed to deserialize transaction: {e}"))
            })?;

        let v1_message = Self::as_v1_message(&transaction.message)?;

        transaction.message.sanitize().map_err(|e| {
            KoraError::InvalidTransaction(format!("Invalid transaction message: {e}"))
        })?;

        Self::validate_resource_limits(v1_message)?;

        Ok(transaction)
    }

    pub fn as_v1_message(message: &VersionedMessage) -> Result<&v1::Message, KoraError> {
        match message {
            VersionedMessage::V1(message) => Ok(message),
            _ => {
                Err(KoraError::InvalidTransaction("Only V1 transactions are supported".to_string()))
            }
        }
    }

    /// A limit a V1 transaction config leaves unset is zero, and zero compute units or zero
    /// loadable account bytes cannot run any instruction. Rejecting that here means the
    /// caller gets an actionable error before Kora simulates, signs, or pays for a
    /// transaction that cannot execute.
    fn validate_resource_limits(v1_message: &v1::Message) -> Result<(), KoraError> {
        let mut unset_limits = Vec::new();
        if v1_message.config.compute_unit_limit.unwrap_or(0) == 0 {
            unset_limits.push("compute_unit_limit");
        }
        if v1_message.config.loaded_accounts_data_size_limit.unwrap_or(0) == 0 {
            unset_limits.push("loaded_accounts_data_size_limit");
        }

        if !unset_limits.is_empty() {
            return Err(KoraError::InvalidTransaction(format!(
                "V1 transaction config must set {} to a non-zero value",
                unset_limits.join(" and ")
            )));
        }

        Ok(())
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
        let serialized = Self::serialize_versioned_transaction(transaction)?;
        Ok(STANDARD.encode(serialized))
    }

    pub fn serialize_versioned_transaction(
        transaction: &VersionedTransaction,
    ) -> Result<Vec<u8>, KoraError> {
        wincode::serialize(transaction).map_err(|_| {
            KoraError::SerializationError("Failed to serialize transaction.".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::KoraError,
        tests::transaction_mock::{create_mock_encoded_v1_transaction, create_test_message},
    };
    use solana_message::{v0, v1, Message};
    use solana_sdk::{
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer as _,
    };

    fn create_v1_message(keypair: &Keypair, data: Vec<u8>) -> v1::Message {
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &data,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        v1::Message::try_compile_with_config(
            &keypair.pubkey(),
            &[instruction],
            Hash::new_unique(),
            v1_resource_limits(),
        )
        .unwrap()
    }

    /// V1 resource limits have to be set explicitly for a transaction to be executable, so
    /// every V1 fixture that is meant to be valid carries them.
    fn v1_resource_limits() -> v1::TransactionConfig {
        v1::TransactionConfig::empty()
            .with_compute_unit_limit(200_000)
            .with_loaded_accounts_data_size_limit(64 * 1024)
    }

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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);

        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message.clone());

        assert_eq!(transaction.signatures.len(), message.header().num_required_signatures as usize);
        for sig in &transaction.signatures {
            assert_eq!(*sig, Signature::default());
        }
        assert_eq!(transaction.message, message);
    }

    #[test]
    fn test_decode_b64_transaction_rejects_legacy_and_v0() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let legacy_message = Message::new(&[instruction.clone()], Some(&keypair.pubkey()));
        let v0_message =
            v0::Message::try_compile(&keypair.pubkey(), &[instruction], &[], Hash::default())
                .unwrap();

        for message in [VersionedMessage::Legacy(legacy_message), VersionedMessage::V0(v0_message)]
        {
            let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();
            let encoded = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();
            assert_eq!(
                TransactionUtil::decode_b64_transaction(&encoded).unwrap_err(),
                KoraError::InvalidTransaction("Only V1 transactions are supported".to_string())
            );
        }
    }

    #[test]
    fn test_decode_b64_transaction_v1_roundtrip() {
        let keypair = Keypair::new();
        let message = VersionedMessage::V1(create_v1_message(&keypair, vec![1, 2, 3]));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let encoded = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();
        let decoded = TransactionUtil::decode_b64_transaction(&encoded).unwrap();

        assert!(matches!(decoded.message, VersionedMessage::V1(_)));
        assert_eq!(decoded, transaction);

        // The signature must verify over the v1 wire message bytes, proving the
        // sign → encode → decode pipeline agrees on the signable payload.
        assert!(
            decoded.signatures[0].verify(keypair.pubkey().as_ref(), &decoded.message.serialize())
        );
    }

    #[test]
    fn test_decode_b64_transaction_v1_rejects_oversized() {
        let keypair = Keypair::new();
        let message = VersionedMessage::V1(create_v1_message(&keypair, vec![0; 4200]));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let encoded = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();
        let result = TransactionUtil::decode_b64_transaction(&encoded);

        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }

    #[test]
    fn test_decode_b64_transaction_v1_rejects_unset_resource_limits() {
        let encoded = create_mock_encoded_v1_transaction(v1::TransactionConfig::empty());

        let error = TransactionUtil::decode_b64_transaction(&encoded)
            .expect_err("an empty V1 config requests zero compute units and zero loaded bytes");
        let error_message = error.to_string();
        assert!(error_message.contains("compute_unit_limit"), "unexpected error: {error_message}");
        assert!(
            error_message.contains("loaded_accounts_data_size_limit"),
            "unexpected error: {error_message}"
        );
    }

    #[test]
    fn test_decode_b64_transaction_v1_rejects_unset_loaded_accounts_data_size_limit() {
        let encoded = create_mock_encoded_v1_transaction(
            v1::TransactionConfig::empty().with_compute_unit_limit(200_000),
        );

        let error = TransactionUtil::decode_b64_transaction(&encoded)
            .expect_err("a V1 config without a data size limit requests zero loaded bytes");
        assert!(
            error.to_string().contains("loaded_accounts_data_size_limit"),
            "unexpected error: {error}"
        );
    }
}
