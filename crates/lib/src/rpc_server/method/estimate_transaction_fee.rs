use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    fee::fee::FeeConfigUtil,
    state::{get_config, get_request_signer_with_signer_key},
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
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
    pub fee_in_token: Option<f64>,
    /// Public key of the signer used for fee estimation (for client consistency)
    pub signer_pubkey: String,
    /// Public key of the payment destination
    pub payment_address: String,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;

    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let config = get_config()?;
    let payment_destination = config.kora.get_payment_address(&signer.solana_pubkey())?;

    let validation_config = &config.validation;
    let fee_payer = signer.solana_pubkey();

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction, rpc_client).await?;

    let fee_in_lamports = FeeConfigUtil::estimate_transaction_fee(
        rpc_client,
        &mut resolved_transaction,
        &fee_payer,
        validation_config.is_payment_required(),
    )
    .await?;

    let mut fee_in_token = None;

    // If fee_token is provided, calculate the fee in that token
    if let Some(fee_token) = &request.fee_token {
        let token_mint = Pubkey::from_str(fee_token).map_err(|_| {
            KoraError::InvalidTransaction("Invalid fee token mint address".to_string())
        })?;

        if !validation_config.supports_token(fee_token) {
            return Err(KoraError::InvalidRequest(format!("Token {fee_token} is not supported")));
        }

        let fee_value_in_token = TokenUtil::calculate_lamports_value_in_token(
            fee_in_lamports,
            &token_mint,
            &validation_config.price_source,
            rpc_client,
        )
        .await?;

        fee_in_token = Some(fee_value_in_token);
    }

    Ok(EstimateTransactionFeeResponse {
        fee_in_lamports,
        fee_in_token,
        signer_pubkey: fee_payer.to_string(),
        payment_address: payment_destination.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        common::{setup_or_get_test_config, setup_or_get_test_signer, RpcMockBuilder},
        transaction_mock::create_mock_encoded_transaction,
    };

    #[tokio::test]
    async fn test_estimate_transaction_fee_decode_error() {
        let _ = setup_or_get_test_config();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateTransactionFeeRequest {
            transaction: "invalid_base64!@#$".to_string(),
            fee_token: None,
            signer_key: None,
        };

        let result = estimate_transaction_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with decode error");
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_invalid_signer_key() {
        let _ = setup_or_get_test_config();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateTransactionFeeRequest {
            transaction: create_mock_encoded_transaction(),
            fee_token: None,
            signer_key: Some("invalid_pubkey".to_string()),
        };

        let result = estimate_transaction_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::ValidationError(_)), "Should return ValidationError");
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_invalid_token_mint() {
        let _ = setup_or_get_test_config();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateTransactionFeeRequest {
            transaction: create_mock_encoded_transaction(),
            fee_token: Some("invalid_mint_address".to_string()),
            signer_key: None,
        };

        let result = estimate_transaction_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid token mint");
        let error = result.unwrap_err();

        assert!(
            matches!(error, KoraError::InvalidTransaction(_)),
            "Should return InvalidTransaction error due to invalid mint parsing"
        );
    }
}
