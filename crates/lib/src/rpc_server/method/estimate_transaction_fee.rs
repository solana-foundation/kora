use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    fee::fee::FeeConfigUtil,
    get_signer,
    state::get_config,
    token::token::TokenUtil,
    transaction::{TransactionUtil, VersionedTransactionResolved},
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
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;

    let signer = get_signer()?;
    let validation_config = &get_config()?.validation;
    let fee_payer = signer.solana_pubkey();

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction, rpc_client).await?;

    let fee_in_lamports = FeeConfigUtil::estimate_transaction_fee(
        rpc_client,
        &mut resolved_transaction,
        Some(&fee_payer),
    )
    .await?;

    let mut fee_in_token = None;

    // If fee_token is provided, calculate the fee in that token
    if let Some(fee_token) = &request.fee_token {
        let token_mint = Pubkey::from_str(fee_token).map_err(|_| {
            KoraError::InvalidTransaction("Invalid fee token mint address".to_string())
        })?;

        let fee_value_in_token = TokenUtil::calculate_lamports_value_in_token(
            fee_in_lamports,
            &token_mint,
            &validation_config.price_source,
            rpc_client,
        )
        .await?;

        fee_in_token = Some(fee_value_in_token);
    }

    Ok(EstimateTransactionFeeResponse { fee_in_lamports, fee_in_token })
}
