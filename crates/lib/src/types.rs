use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    transaction::{decode_b58_transaction, decode_b64_transaction, encode_transaction_b58, encode_transaction_b64},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionEncoding {
    Base58,
    Base64,
}

impl Default for TransactionEncoding {
    fn default() -> Self {
        TransactionEncoding::Base58
    }
}

impl TransactionEncoding {
    pub fn decode_transaction(&self, encoded: &str) -> Result<Transaction, KoraError> {
        match self {
            TransactionEncoding::Base58 => decode_b58_transaction(encoded),
            TransactionEncoding::Base64 => decode_b64_transaction(encoded),
        }
    }

    pub fn encode_transaction(&self, transaction: &Transaction) -> Result<String, KoraError> {
        match self {
            TransactionEncoding::Base58 => encode_transaction_b58(transaction),
            TransactionEncoding::Base64 => encode_transaction_b64(transaction),
        }
    }
} 