use crate::{
    bundle::{BundleError, BundleProcessor, JitoError},
    error::KoraError,
    fee::fee::FeeConfigUtil,
    rpc_server::middleware_utils::default_sig_verify,
    state::get_request_signer_with_signer_key,
    validator::bundle_validator::BundleValidator,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateBundleFeeRequest {
    /// Array of base64-encoded transactions
    pub transactions: Vec<String>,
    #[serde(default)]
    pub fee_token: Option<String>,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// Optional indices of transactions to estimate fees for (defaults to all if not specified)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_only_indices: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateBundleFeeResponse {
    pub fee_in_lamports: u64,
    pub fee_in_token: Option<u64>,
    /// Public key of the signer used for fee estimation (for client consistency)
    pub signer_pubkey: String,
    /// Public key of the payment destination
    pub payment_address: String,
}

pub async fn estimate_bundle_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateBundleFeeRequest,
) -> Result<EstimateBundleFeeResponse, KoraError> {
    let config = &get_config()?;

    if !config.kora.bundle.enabled {
        return Err(BundleError::Jito(JitoError::NotEnabled).into());
    }

    // Validate bundle size on ALL transactions first
    BundleValidator::validate_jito_bundle_size(&request.transactions)?;

    // Extract only the transactions we need to process
    let (transactions_to_process, _index_to_position) =
        BundleProcessor::extract_transactions_to_process(
            &request.transactions,
            request.sign_only_indices,
        )?;

    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let fee_payer = signer.pubkey();
    let payment_destination = config.kora.get_payment_address(&fee_payer)?;

    let processor = BundleProcessor::process_bundle(
        &transactions_to_process,
        fee_payer,
        &payment_destination,
        config,
        rpc_client,
        request.sig_verify,
    )
    .await?;

    let fee_in_lamports = processor.total_required_lamports;

    // Calculate fee in token if requested
    let fee_in_token = FeeConfigUtil::calculate_fee_in_token(
        fee_in_lamports,
        request.fee_token.as_deref(),
        rpc_client,
        config,
    )
    .await?;

    Ok(EstimateBundleFeeResponse {
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
        common::{setup_or_get_test_signer, setup_or_get_test_usage_limiter, RpcMockBuilder},
        config_mock::ConfigMockBuilder,
        transaction_mock::create_mock_encoded_transaction,
    };

    #[tokio::test]
    async fn test_estimate_bundle_fee_empty_bundle() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec![],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with empty bundle");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_disabled() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(false).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["some_tx".to_string()],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail when bundles disabled");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("not enabled"));
        }
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_too_large() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["tx".to_string(); 6],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with too many transactions");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("maximum size"));
        }
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_invalid_signer_key() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["some_tx".to_string()],
            fee_token: None,
            signer_key: Some("invalid_pubkey".to_string()),
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_exactly_max_size() {
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build_and_setup();
        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // 5 transactions is the maximum allowed
        let transactions: Vec<String> = (0..5).map(|_| create_mock_encoded_transaction()).collect();

        let request = EstimateBundleFeeRequest {
            transactions,
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transactions");
        let response = result.unwrap();
        assert!(response.fee_in_lamports > 0);
        assert!(!response.signer_pubkey.is_empty());
        assert!(!response.payment_address.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_single_transaction() {
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build_and_setup();
        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // Single transaction bundle is valid
        let request = EstimateBundleFeeRequest {
            transactions: vec![create_mock_encoded_transaction()],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transaction");
        let response = result.unwrap();
        assert!(response.fee_in_lamports > 0);
        assert!(!response.signer_pubkey.is_empty());
        assert!(!response.payment_address.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_sig_verify_default() {
        // Test that sig_verify defaults correctly via serde (defaults to false)
        let json = r#"{"transactions": ["tx1"]}"#;
        let request: EstimateBundleFeeRequest = serde_json::from_str(json).unwrap();

        assert!(!request.sig_verify, "sig_verify should default to false");
        assert!(request.signer_key.is_none());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_request_deserialization() {
        let json = r#"{
            "transactions": ["tx1", "tx2"],
            "fee_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "signer_key": "11111111111111111111111111111111",
            "sig_verify": false
        }"#;
        let request: EstimateBundleFeeRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.transactions.len(), 2);
        assert_eq!(
            request.fee_token,
            Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string())
        );
        assert_eq!(request.signer_key, Some("11111111111111111111111111111111".to_string()));
        assert!(!request.sig_verify);
    }
}
