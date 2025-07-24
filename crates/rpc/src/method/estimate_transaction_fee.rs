use std::sync::Arc;
use utoipa::ToSchema;

use kora_lib::{
    error::KoraError,
    transaction::{
        decode_b64_transaction, estimate_transaction_fee as lib_estimate_transaction_fee,
        VersionedTransactionResolved,
    },
};

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeRequest {
    pub transaction: String, // Base64 encoded serialized transaction
    pub fee_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = decode_b64_transaction(&request.transaction)?;

    // Resolve lookup tables for V0 transactions to ensure accurate fee calculation
    let mut resolved_transaction = VersionedTransactionResolved::new(&transaction);
    resolved_transaction.resolve_addresses(rpc_client).await?;

    let fee = lib_estimate_transaction_fee(rpc_client, &resolved_transaction).await?;

    Ok(EstimateTransactionFeeResponse { fee_in_lamports: fee })
}
