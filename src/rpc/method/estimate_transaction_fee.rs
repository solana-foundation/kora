use std::sync::Arc;

use crate::rpc::{error::KoraError, response::KoraResponse};

use solana_client::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;

#[derive(Debug)]
pub struct TransactionEstimate {
    pub fee_in_lamports: u64,
}

pub type EstimateTransactionFeeResponse = KoraResponse<TransactionEstimate>;

pub async fn estimate_transaction_fee(
    rpc_client: Arc<RpcClient>,
    transaction: Transaction,
) -> Result<EstimateTransactionFeeResponse, KoraError> {

    // Get prio fee from tx accounts
    let addresses =transaction.message.account_keys; 

    let prio_fee = rpc_client
        .get_recent_prioritization_fees(&addresses)
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    let fees =  prio_fee.iter().map(|fee| fee.prioritization_fee).sum::<u64>();
    
    let avg_fee = fees / prio_fee.len() as u64;

    let estimate = TransactionEstimate {
        fee_in_lamports: avg_fee,
    };

    Ok(KoraResponse::ok(estimate))
}
