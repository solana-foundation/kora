use std::sync::Arc;
use utoipa::ToSchema;

use kora_lib::{
    config::ValidationConfig,
    error::KoraError,
    token::calculate_lamports_value_in_token,
    transaction::{
        decode_b64_transaction, estimate_transaction_fee as lib_estimate_transaction_fee,
        VersionedTransactionResolved,
    },
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeRequest {
    pub transaction: String, // Base64 encoded serialized transaction
    #[serde(default)]
    pub fee_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
    pub fee_in_token: Option<f64>,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = decode_b64_transaction(&request.transaction)?;

    // Resolve lookup tables for V0 transactions to ensure accurate fee calculation
    let mut resolved_transaction = VersionedTransactionResolved::new(&transaction);
    resolved_transaction.resolve_addresses(rpc_client).await?;

    let fee_in_lamports = lib_estimate_transaction_fee(rpc_client, &resolved_transaction).await?;

    let mut fee_in_token = None;

    // If fee_token is provided, calculate the fee in that token
    if let Some(fee_token) = &request.fee_token {
        let token_mint = Pubkey::from_str(fee_token).map_err(|_| {
            KoraError::InvalidTransaction("Invalid fee token mint address".to_string())
        })?;

        let fee_value_in_token = calculate_lamports_value_in_token(
            fee_in_lamports,
            &token_mint,
            &validation.price_source,
            rpc_client,
        )
        .await?;

        fee_in_token = Some(fee_value_in_token);
    }

    Ok(EstimateTransactionFeeResponse { fee_in_lamports, fee_in_token })
}
