use crate::{
    bundle::{BundleError, BundleProcessingMode, BundleProcessor, JitoError},
    plugin::PluginExecutionContext,
    rpc_server::middleware_utils::default_sig_verify,
    transaction::TransactionUtil,
    validator::bundle_validator::BundleValidator,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

#[cfg(not(test))]
use crate::state::{get_config, get_request_signer_with_signer_key};

#[cfg(test)]
use crate::state::get_request_signer_with_signer_key;
#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignBundleRequest {
    /// Array of base64-encoded transactions
    pub transactions: Vec<String>,
    /// Optional signer key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Optional indices of transactions to sign (defaults to all if not specified)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_only_indices: Option<Vec<usize>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignBundleResponse {
    /// Array of base64-encoded signed transactions
    pub signed_transactions: Vec<String>,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_bundle(
    rpc_client: &Arc<RpcClient>,
    request: SignBundleRequest,
) -> Result<SignBundleResponse, KoraError> {
    let SignBundleRequest { transactions, signer_key, sig_verify, user_id, sign_only_indices } =
        request;
    let config = &get_config()?;

    if !config.kora.bundle.enabled {
        return Err(BundleError::Jito(JitoError::NotEnabled).into());
    }

    // Validate bundle size on ALL transactions first
    BundleValidator::validate_jito_bundle_size(&transactions)?;

    // Extract only the transactions we need to process
    let (transactions_to_process, index_to_position) =
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
        Some(PluginExecutionContext::SignBundle),
        BundleProcessingMode::CheckUsage(user_id.as_deref()),
    )
    .await?;

    let signed_resolved =
        processor.sign_all(&signer, &fee_payer, rpc_client, config, false).await?;

    // Encode signed transactions
    let encoded_signed: Vec<String> = signed_resolved
        .iter()
        .map(|r| TransactionUtil::encode_versioned_transaction(&r.transaction))
        .collect::<Result<Vec<_>, _>>()?;

    // Merge signed transactions back into original positions
    let signed_transactions = BundleProcessor::merge_signed_transactions(
        &transactions,
        encoded_signed,
        &index_to_position,
    );

    let signed_indices = BundleValidator::signed_indices_for_bundle(
        transactions.len(),
        sign_only_indices.as_deref(),
    );
    BundleValidator::simulate_and_validate_sequential_bundle(
        rpc_client,
        config,
        &signed_transactions,
        &signed_indices,
        &fee_payer,
        !sig_verify,
    )
    .await?;

    Ok(SignBundleResponse { signed_transactions, signer_pubkey: fee_payer.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fee::price::{PriceConfig, PriceModel},
        tests::{
            common::{setup_or_get_test_signer, setup_or_get_test_usage_limiter, RpcMockBuilder},
            config_mock::{
                mock_state::setup_config_mock, ConfigMockBuilder, ValidationConfigBuilder,
            },
        },
        transaction::TransactionUtil,
    };
    use mockito::{Matcher, Server};
    use serde_json::json;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::pubkey::Pubkey;
    use solana_system_interface::instruction::transfer;

    #[tokio::test]
    async fn test_sign_bundle_empty_bundle() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignBundleRequest {
            transactions: vec![],
            signer_key: None,
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with empty bundle");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn test_sign_bundle_disabled() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(false).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignBundleRequest {
            transactions: vec!["some_tx".to_string()],
            signer_key: None,
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail when bundles disabled");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("not enabled"));
        }
    }

    #[tokio::test]
    async fn test_sign_bundle_too_large() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignBundleRequest {
            transactions: vec!["tx".to_string(); 6],
            signer_key: None,
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with too many transactions");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("maximum size"));
        }
    }

    #[tokio::test]
    async fn test_sign_bundle_invalid_signer_key() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignBundleRequest {
            transactions: vec!["some_tx".to_string()],
            signer_key: Some("invalid_pubkey".to_string()),
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_sign_bundle_exactly_max_size() {
        let mut validation = ValidationConfigBuilder::new()
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .build();
        validation.price = PriceConfig { model: PriceModel::Free };
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .with_validation(validation)
            .build_and_setup();
        let signer_pubkey = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(
            RpcMockBuilder::new()
                .with_fee_estimate(5000)
                .with_blockhash()
                .with_simulation()
                .build(),
        );

        // Create transactions with signer as fee payer

        let transactions: Vec<String> = (0..5)
            .map(|_| {
                let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000000000);
                let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&signer_pubkey)));
                let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
                TransactionUtil::encode_versioned_transaction(&transaction).unwrap()
            })
            .collect();

        // Use signer_key to ensure consistency - prevents race conditions with parallel tests
        let request = SignBundleRequest {
            transactions,
            signer_key: Some(signer_pubkey.to_string()),
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transactions");
        let response = result.unwrap();
        assert_eq!(response.signed_transactions.len(), 5);
        assert!(!response.signer_pubkey.is_empty());
    }

    #[tokio::test]
    async fn test_sign_bundle_single_transaction() {
        let mut validation = ValidationConfigBuilder::new()
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .build();
        validation.price = PriceConfig { model: PriceModel::Free };
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .with_validation(validation)
            .build_and_setup();
        let signer_pubkey = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(
            RpcMockBuilder::new()
                .with_fee_estimate(5000)
                .with_blockhash()
                .with_simulation()
                .build(),
        );

        // Create transaction with signer as fee payer
        let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000000000);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&signer_pubkey)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let encoded_tx = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();

        // Single transaction bundle is valid
        let request = SignBundleRequest {
            transactions: vec![encoded_tx],
            signer_key: Some(signer_pubkey.to_string()),
            sig_verify: true,
            user_id: None,
            sign_only_indices: None,
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transaction");
        let response = result.unwrap();
        assert_eq!(response.signed_transactions.len(), 1);
        assert!(!response.signer_pubkey.is_empty());
    }

    #[tokio::test]
    async fn test_sign_bundle_sig_verify_default() {
        // Test that sig_verify defaults correctly via serde (defaults to false)
        let json = r#"{"transactions": ["tx1"]}"#;
        let request: SignBundleRequest = serde_json::from_str(json).unwrap();

        assert!(!request.sig_verify, "sig_verify should default to false");
        assert!(request.signer_key.is_none());
    }

    #[tokio::test]
    async fn test_sign_bundle_request_deserialization() {
        let json = r#"{
            "transactions": ["tx1", "tx2"],
            "signer_key": "11111111111111111111111111111111",
            "sig_verify": false,
            "user_id": "test-user-456"
        }"#;
        let request: SignBundleRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.transactions.len(), 2);
        assert_eq!(request.signer_key, Some("11111111111111111111111111111111".to_string()));
        assert!(!request.sig_verify);
        assert_eq!(request.user_id, Some("test-user-456".to_string()));
        assert!(request.sign_only_indices.is_none());
    }

    #[tokio::test]
    async fn test_sign_bundle_request_deserialization_with_sign_only_indices() {
        let json = r#"{
            "transactions": ["tx1", "tx2", "tx3"],
            "signer_key": "11111111111111111111111111111111",
            "sig_verify": false,
            "sign_only_indices": [0, 2]
        }"#;
        let request: SignBundleRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.transactions.len(), 3);
        assert_eq!(request.signer_key, Some("11111111111111111111111111111111".to_string()));
        assert!(!request.sig_verify);
        assert_eq!(request.sign_only_indices, Some(vec![0, 2]));
    }

    #[tokio::test]
    async fn test_sign_bundle_rejects_sequential_outflow_violation() {
        let mut server = Server::new_async().await;
        let simulate_mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({"method": "simulateBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"value":{"summary":"succeeded","transactionResults":[{"err":null,"logs":["Program 11111111111111111111111111111111 invoke [1]"]},{"err":null,"logs":["Program 11111111111111111111111111111111 invoke [1]"],"preExecutionAccounts":[{"lamports":2500000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}],"postExecutionAccounts":[{"lamports":1000000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}]},{"err":null,"logs":["Program 11111111111111111111111111111111 invoke [1]"]}]}}}"#,
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

        let make_non_signed_tx = || {
            let payer = Pubkey::new_unique();
            let ix = transfer(&payer, &Pubkey::new_unique(), 1_000_000_000);
            let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer)));
            let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
            TransactionUtil::encode_versioned_transaction(&transaction).unwrap()
        };

        let signed_ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1_000_000_000);
        let signed_message =
            VersionedMessage::Legacy(Message::new(&[signed_ix], Some(&signer_pubkey)));
        let signed_transaction =
            TransactionUtil::new_unsigned_versioned_transaction(signed_message);
        let signed_encoded =
            TransactionUtil::encode_versioned_transaction(&signed_transaction).unwrap();

        let transactions = vec![make_non_signed_tx(), signed_encoded, make_non_signed_tx()];
        let request = SignBundleRequest {
            transactions,
            signer_key: Some(signer_pubkey.to_string()),
            sig_verify: true,
            user_id: None,
            sign_only_indices: Some(vec![1]),
        };

        let result = sign_bundle(&rpc_client, request).await;

        simulate_mock.assert();
        assert!(result.is_err(), "Expected sequential outflow validation error");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Total transfer amount"), "Unexpected error: {err}");
        assert!(err.contains("exceeds maximum allowed"), "Unexpected error: {err}");
    }
}
