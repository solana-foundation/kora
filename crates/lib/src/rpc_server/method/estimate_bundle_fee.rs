use crate::{
    bundle::{BundleError, BundleProcessingMode, BundleProcessor, JitoError},
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
    let EstimateBundleFeeRequest {
        transactions,
        fee_token,
        signer_key,
        sig_verify,
        sign_only_indices,
    } = request;
    let config = &get_config()?;

    if !config.kora.bundle.enabled {
        return Err(BundleError::Jito(JitoError::NotEnabled).into());
    }

    // Validate bundle size on ALL transactions first
    BundleValidator::validate_jito_bundle_size(&transactions)?;

    // Extract only the transactions we need to process
    let (transactions_to_process, _index_to_position) =
        BundleProcessor::extract_transactions_to_process(&transactions, sign_only_indices.clone())?;

    let signer = get_request_signer_with_signer_key(signer_key.as_deref())?;
    let fee_payer = signer.pubkey();
    let payment_destination = config.kora.get_payment_address(&fee_payer)?;

    let sig_verify = sig_verify || config.kora.force_sig_verify;
    let processor = BundleProcessor::process_bundle(
        &transactions_to_process,
        fee_payer,
        &payment_destination,
        config,
        rpc_client,
        sig_verify,
        None,
        BundleProcessingMode::SkipUsage,
    )
    .await?;

    let fee_in_lamports = processor.total_required_lamports;

    let signed_indices = BundleValidator::signed_indices_for_bundle(
        transactions.len(),
        sign_only_indices.as_deref(),
    );
    BundleValidator::simulate_and_validate_sequential_bundle(
        rpc_client,
        config,
        &transactions,
        &signed_indices,
        &fee_payer,
        true,
    )
    .await?;

    // Calculate fee in token if requested
    let fee_in_token = FeeConfigUtil::calculate_fee_in_token(
        fee_in_lamports,
        fee_token.as_deref(),
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
    use crate::{
        config::TransactionPluginType,
        fee::price::{PriceConfig, PriceModel},
        tests::{
            common::{setup_or_get_test_signer, setup_or_get_test_usage_limiter, RpcMockBuilder},
            config_mock::{mock_state::setup_config_mock, ConfigMockBuilder},
            transaction_mock::create_mock_encoded_transaction,
        },
        transaction::TransactionUtil,
    };
    use mockito::{Matcher, Server};
    use serde_json::json;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::pubkey::Pubkey;
    use solana_system_interface::instruction::transfer;

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

    #[tokio::test]
    async fn test_estimate_bundle_fee_skips_plugins() {
        let mut config = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build();
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        let _m = setup_config_mock(config);

        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // Not gas_swap-compatible shape; would fail if plugins ran during estimate.
        let request = EstimateBundleFeeRequest {
            transactions: vec![create_mock_encoded_transaction()],
            fee_token: None,
            signer_key: None,
            sig_verify: false,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;
        assert!(result.is_ok(), "estimateBundleFee should skip transaction plugins");
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_rejects_sequential_outflow_violation() {
        let mut server = Server::new_async().await;
        let simulate_mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({"method": "simulateBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"value":{"summary":"succeeded","transactionResults":[{"err":null,"logs":["Program 11111111111111111111111111111111 invoke [1]"],"preExecutionAccounts":[{"lamports":2500000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}],"postExecutionAccounts":[{"lamports":1000000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}]}]}}}"#,
            )
            .create();

        let mut config = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .with_max_allowed_lamports(1_000_000)
            .build();
        config.validation.price = PriceConfig { model: PriceModel::Free };
        config.kora.bundle.jito.block_engine_url = server.url();
        config.kora.bundle.jito.simulate_bundle_url = Some(server.url());
        let _m = setup_config_mock(config);
        let _ = setup_or_get_test_usage_limiter().await;

        let signer_pubkey = setup_or_get_test_signer();
        let rpc_client = Arc::new(
            RpcMockBuilder::new()
                .with_fee_estimate(5000)
                .with_blockhash()
                .with_simulation()
                .build(),
        );

        let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1_000_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&signer_pubkey)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let encoded_tx = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();

        let request = EstimateBundleFeeRequest {
            transactions: vec![encoded_tx],
            fee_token: None,
            signer_key: Some(signer_pubkey.to_string()),
            sig_verify: false,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        simulate_mock.assert();
        assert!(result.is_err(), "Expected sequential outflow validation error");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Total transfer amount"), "Unexpected error: {err}");
        assert!(err.contains("exceeds maximum allowed"), "Unexpected error: {err}");
    }
}
