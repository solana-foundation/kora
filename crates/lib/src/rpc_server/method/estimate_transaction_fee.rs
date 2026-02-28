use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    fee::fee::FeeConfigUtil,
    rpc_server::middleware_utils::default_sig_verify,
    state::get_request_signer_with_signer_key,
    transaction::{TransactionUtil, VersionedTransactionResolved},
};

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Request payload for estimating the fee of a transaction.
///
/// This endpoint calculates the required fee for a given transaction based on current
/// oracle prices and Kora's configuration. It can estimate fees in native SOL (lamports)
/// or in a specific supported SPL token if `fee_token` is provided.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeRequest {
    /// Base64-encoded serialized transaction
    pub transaction: String,
    /// Optional mint address of the SPL token to calculate the fee in. If omitted, returns only the lamport fee.
    #[serde(default)]
    pub fee_token: Option<String>,
    /// Optional public key of the signer to ensure consistency and prevent unauthorized estimation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to true)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
}

/// Response payload containing the estimated transaction fee.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeResponse {
    /// The exact fee required in native SOL (lamports)
    pub fee_in_lamports: u64,
    /// The estimated fee in the requested SPL token (if `fee_token` was provided in the request)
    pub fee_in_token: Option<u64>,
    /// Public key of the signer used for fee estimation (for client consistency)
    pub signer_pubkey: String,
    /// Public key of the payment destination where the fee should be sent
    pub payment_address: String,
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;

    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let config = &get_config()?;
    let payment_destination = config.kora.get_payment_address(&signer.pubkey())?;

    let validation_config = &config.validation;
    let fee_payer = signer.pubkey();

    let mut resolved_transaction = VersionedTransactionResolved::from_transaction(
        &transaction,
        config,
        rpc_client,
        request.sig_verify,
    )
    .await?;

    let fee_calculation = FeeConfigUtil::estimate_kora_fee(
        &mut resolved_transaction,
        &fee_payer,
        validation_config.is_payment_required(),
        rpc_client,
        config,
    )
    .await?;

    let fee_in_lamports = fee_calculation.total_fee_lamports;

    #[allow(clippy::needless_borrow)]
    // Calculate fee in token if requested
    let fee_in_token = FeeConfigUtil::calculate_fee_in_token(
        fee_in_lamports,
        request.fee_token.as_deref(),
        rpc_client,
        &config,
    )
    .await?;

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
            sig_verify: true,
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
            sig_verify: true,
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
            sig_verify: true,
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
