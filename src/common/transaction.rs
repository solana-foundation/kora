use solana_sdk::transaction::Transaction;

use crate::common::KoraError;

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
