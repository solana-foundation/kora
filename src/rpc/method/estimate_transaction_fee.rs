use std::sync::Arc;

use crate::common::error::KoraError;

use bincode;
use bs58;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeRequest {
    pub transaction_data: String, // Base58 encoded serialized transaction
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    // Deserialize the transaction from base58
    log::info!(
        "Called estimate_transaction_fee with transaction data: {}",
        request.transaction_data
    );
    let decoded_bytes = match bs58::decode(&request.transaction_data).into_vec() {
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

    // Get prio fee from tx accounts
    let addresses = transaction.message.account_keys;

    let prio_fee = match rpc_client.get_recent_prioritization_fees(&addresses).await {
        Ok(fees) => fees,
        Err(e) => return Err(KoraError::RpcError(e.to_string())),
    };

    let fees = prio_fee.iter().map(|fee| fee.prioritization_fee).sum::<u64>();

    let avg_fee = fees / prio_fee.len() as u64;

    let estimate = EstimateTransactionFeeResponse { fee_in_lamports: avg_fee };

    Ok(estimate)
}
