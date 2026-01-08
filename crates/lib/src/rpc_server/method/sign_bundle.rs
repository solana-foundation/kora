use crate::{
    bundle::{BundleError, BundleProcessor, JitoError},
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
    /// Whether to verify signatures during simulation (defaults to true)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
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
    BundleValidator::validate_jito_bundle_size(&request.transactions)?;

    let config = &get_config()?;

    if !config.kora.bundle.enabled {
        return Err(BundleError::Jito(JitoError::NotEnabled).into());
    }

    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let fee_payer = signer.pubkey();
    let payment_destination = config.kora.get_payment_address(&fee_payer)?;

    let processor = BundleProcessor::process_bundle(
        &request.transactions,
        fee_payer,
        &payment_destination,
        config,
        rpc_client,
        request.sig_verify,
    )
    .await?;

    let signed_resolved = processor.sign_all(&signer, &fee_payer, rpc_client).await?;

    let signed_transactions = signed_resolved
        .iter()
        .map(|r| TransactionUtil::encode_versioned_transaction(&r.transaction))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SignBundleResponse { signed_transactions, signer_pubkey: fee_payer.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        common::{setup_or_get_test_signer, RpcMockBuilder},
        config_mock::ConfigMockBuilder,
    };

    #[tokio::test]
    async fn test_sign_bundle_empty_bundle() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request =
            SignBundleRequest { transactions: vec![], signer_key: None, sig_verify: true };

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
        };

        let result = sign_bundle(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_sign_bundle_exactly_max_size() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        // 5 transactions is the maximum allowed
        let request = SignBundleRequest {
            transactions: vec!["tx".to_string(); 5],
            signer_key: None,
            sig_verify: true,
        };

        let result = sign_bundle(&rpc_client, request).await;

        // Will fail at decoding stage (not size validation), which is expected
        // This test verifies that 5 transactions passes size validation
        assert!(result.is_err());
        // Should NOT be a JitoError about bundle size
        if let KoraError::JitoError(msg) = &result.unwrap_err() {
            assert!(
                !msg.contains("maximum size"),
                "5 transactions should not fail size validation"
            );
        }
    }

    #[tokio::test]
    async fn test_sign_bundle_single_transaction() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        // Single transaction bundle is valid
        let request = SignBundleRequest {
            transactions: vec!["tx".to_string()],
            signer_key: None,
            sig_verify: true,
        };

        let result = sign_bundle(&rpc_client, request).await;

        // Will fail at decoding stage, but should pass size validation
        assert!(result.is_err());
        // Should NOT be an empty bundle error
        let err = result.unwrap_err();
        assert!(!matches!(err, KoraError::InvalidTransaction(ref msg) if msg.contains("empty")));
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
            "sig_verify": false
        }"#;
        let request: SignBundleRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.transactions.len(), 2);
        assert_eq!(request.signer_key, Some("11111111111111111111111111111111".to_string()));
        assert!(!request.sig_verify);
    }
}
