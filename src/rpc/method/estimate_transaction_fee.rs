use std::sync::Arc;

use crate::common::{
    error::KoraError, transaction::decode_b58_transaction, LAMPORTS_PER_SIGNATURE,
};

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeRequest {
    pub transaction: String, // Base58 encoded serialized transaction
    pub fee_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = decode_b58_transaction(&request.transaction)?;

    // Get base fee (computation + rent for any account creations)
    let simulation = rpc_client
        .simulate_transaction(&transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // multiple by lamports per signature use sdk constant
    let base_fee = simulation.value.units_consumed.unwrap_or(0) * LAMPORTS_PER_SIGNATURE;

    // Get priority fee from recent blocks
    let priority_stats = rpc_client
        .get_recent_prioritization_fees(&[])
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;
    let priority_fee = priority_stats.iter().map(|fee| fee.prioritization_fee).max().unwrap_or(0);

    Ok(EstimateTransactionFeeResponse { fee_in_lamports: base_fee + priority_fee })
}
