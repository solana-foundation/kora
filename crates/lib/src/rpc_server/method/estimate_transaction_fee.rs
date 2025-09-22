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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateTransactionFeeRequest {
    pub transaction: String, // Base64 encoded serialized transaction
    #[serde(default)]
    pub fee_token: Option<String>,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to true)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
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
    log::error!("RPC Method: estimateTransactionFee - Entry: transaction_len={}, fee_token={:?}, signer_key={:?}, sig_verify={}",
        request.transaction.len(), request.fee_token, request.signer_key, request.sig_verify);

    let transaction = match TransactionUtil::decode_b64_transaction(&request.transaction) {
        Ok(tx) => {
            log::error!("Transaction decoded successfully: signatures={}", tx.signatures.len());
            tx
        }
        Err(e) => {
            log::error!("Transaction decode failed: {e}");
            return Err(e);
        }
    };

    let signer = match get_request_signer_with_signer_key(request.signer_key.as_deref()) {
        Ok(s) => {
            log::error!("Signer obtained: pubkey={}", s.solana_pubkey());
            s
        }
        Err(e) => {
            log::error!("Failed to get signer: {e}");
            return Err(e);
        }
    };

    let config = match get_config() {
        Ok(c) => {
            log::error!("Config loaded successfully");
            c
        }
        Err(e) => {
            log::error!("Failed to get config: {e}");
            return Err(e);
        }
    };

    let fee_payer = signer.solana_pubkey();
    let payment_destination = match config.kora.get_payment_address(&fee_payer) {
        Ok(addr) => {
            log::error!("Payment destination: {addr}");
            addr
        }
        Err(e) => {
            log::error!("Failed to get payment destination: {e}");
            return Err(e);
        }
    };

    let validation_config = &config.validation;
    log::error!(
        "Validation config: payment_required={}, price_source={:?}",
        validation_config.is_payment_required(),
        validation_config.price_source
    );

    log::error!("Resolving transaction with lookup tables");
    let mut resolved_transaction = match VersionedTransactionResolved::from_transaction(
        &transaction,
        rpc_client,
        request.sig_verify,
    )
    .await
    {
        Ok(resolved) => {
            log::error!(
                "Transaction resolved successfully: total_accounts={}, total_instructions={}",
                resolved.all_account_keys.len(),
                resolved.all_instructions.len()
            );
            resolved
        }
        Err(e) => {
            log::error!("Transaction resolution failed: {e}");
            return Err(e);
        }
    };

    log::error!("Estimating Kora fee");
    let fee_calculation = match FeeConfigUtil::estimate_kora_fee(
        rpc_client,
        &mut resolved_transaction,
        &fee_payer,
        validation_config.is_payment_required(),
        Some(validation_config.price_source.clone()),
    )
    .await
    {
        Ok(calc) => {
            log::error!("Fee estimation complete: total_fee_lamports={}", calc.total_fee_lamports);
            calc
        }
        Err(e) => {
            log::error!("Fee estimation failed: {e}");
            return Err(e);
        }
    };

    let fee_in_lamports = fee_calculation.total_fee_lamports;

    log::error!("Calculating fee in token if requested");
    let fee_in_token = match FeeConfigUtil::calculate_fee_in_token(
        rpc_client,
        fee_in_lamports,
        request.fee_token.as_deref(),
    )
    .await
    {
        Ok(token_fee) => {
            log::error!("Token fee calculation result: {token_fee:?}");
            token_fee
        }
        Err(e) => {
            log::error!("Token fee calculation failed: {e}");
            return Err(e);
        }
    };

    log::error!("RPC Method: estimateTransactionFee - Success: fee_in_lamports={fee_in_lamports}, fee_in_token={fee_in_token:?}");

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
