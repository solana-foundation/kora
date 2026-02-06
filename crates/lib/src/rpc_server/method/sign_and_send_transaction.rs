use crate::{rpc_server::middleware_utils::default_sig_verify, usage_limit::UsageTracker};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    state::get_request_signer_with_signer_key,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    KoraError,
    webhook::{emit_event, WebhookEvent},
    webhook::events::{TransactionSignedData, TransactionFailedData},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignAndSendTransactionRequest {
    pub transaction: String,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to true)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignAndSendTransactionResponse {
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_and_send_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignAndSendTransactionRequest,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    let result: Result<SignAndSendTransactionResponse, KoraError> = async {

        let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;

        // Check usage limit for transaction sender
        UsageTracker::check_transaction_usage_limit(&transaction).await?;

        let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;

        let mut resolved_transaction = VersionedTransactionResolved::from_transaction(
            &transaction,
            rpc_client,
            request.sig_verify,
        )
        .await?;

        let (_, signed_transaction) =
            resolved_transaction.sign_and_send_transaction(&signer, rpc_client).await?;
        
        emit_event(WebhookEvent::TransactionSigned(TransactionSignedData {
            transaction_id: signed_transaction.clone(),
            signer_pubkey: signer.pubkey().to_string(),
            method: "signAndSendTransaction".to_string(),
        }))
        .await;

        Ok(SignAndSendTransactionResponse {
            signed_transaction,
            signer_pubkey: signer.pubkey().to_string(),
        })
    }
    .await;

    // Emit failure webhook if error occurred
    if let Err(ref e) = result {
        let signer_pubkey = request
            .signer_key
            .as_deref()
            .and_then(|key| key.parse().ok())
            .map(|pubkey: solana_sdk::pubkey::Pubkey| pubkey.to_string());

        emit_event(WebhookEvent::TransactionFailed(TransactionFailedData {
            error: e.to_string(),
            method: "signTransaction".to_string(),
            signer_pubkey,
        }))
        .await;
    }

    result
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
    async fn test_sign_and_send_transaction_decode_error() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignAndSendTransactionRequest {
            transaction: "invalid_base64!@#$".to_string(),
            signer_key: None,
            sig_verify: true,
        };

        let result = sign_and_send_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with decode error");
    }

    #[tokio::test]
    async fn test_sign_and_send_transaction_invalid_signer_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignAndSendTransactionRequest {
            transaction: create_mock_encoded_transaction(),
            signer_key: Some("invalid_pubkey".to_string()),
            sig_verify: true,
        };

        let result = sign_and_send_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::ValidationError(_)), "Should return ValidationError");
    }
}
