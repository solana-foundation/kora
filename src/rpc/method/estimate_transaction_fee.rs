use std::sync::Arc;

use crate::common::{error::KoraError, jup::get_quote};

use bincode;
use bs58;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;

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
    // Deserialize the transaction from base58
    log::info!("Called estimate_transaction_fee with transaction: {}", request.transaction);
    let decoded_bytes = match bs58::decode(&request.transaction).into_vec() {
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
        Err(e) => {
            log::error!("Failed to get recent prioritization fees: {}", e);
            return Err(KoraError::Rpc(e.to_string()));
        },
    };

    let fees = prio_fee.iter().map(|fee| fee.prioritization_fee).sum::<u64>();

    let avg_fee = fees / prio_fee.len() as u64;

    // Get quote for how much of fee_token we need to swap to get avg_fee amount of SOL
    let quote = match get_quote(request.fee_token, avg_fee).await {
        Ok(quote) => quote,
        Err(e) => {
            log::error!("Failed to get quote: {}", e);
            return Err(KoraError::FeeEstimation);
        }
    };

    // The total fee in the fee_token (e.g., USDC) needed to:
    // 1. Get enough SOL to pay for the transaction fee (quote.in_amount)
    let estimate = EstimateTransactionFeeResponse { 
        fee_in_lamports: quote.in_amount 
    };

    Ok(estimate)
}
