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
    let transaction = match bs58::decode(&request.transaction_data)
        .into_vec()
        .map_err(|e| KoraError::InvalidTransaction(format!("Invalid base58: {}", e)))
        .and_then(|bytes| {
            bincode::deserialize::<Transaction>(&bytes)
                .map_err(|e| KoraError::InvalidTransaction(format!("Invalid transaction: {}", e)))
        }) {
        Ok(tx) => tx,
        Err(e) => return Err(KoraError::InvalidTransaction(e.to_string())),
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
