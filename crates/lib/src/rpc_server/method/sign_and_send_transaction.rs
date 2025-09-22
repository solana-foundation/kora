use crate::{rpc_server::middleware_utils::default_sig_verify, usage_limit::UsageTracker};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    state::get_request_signer_with_signer_key,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    KoraError,
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
    pub signature: String,
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_and_send_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignAndSendTransactionRequest,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    log::error!("RPC Method: signAndSendTransaction - Entry: transaction_len={}, signer_key={:?}, sig_verify={}",
        request.transaction.len(), request.signer_key, request.sig_verify);

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

    log::error!("Checking usage limit for transaction sender");
    if let Err(e) = UsageTracker::check_transaction_usage_limit(&transaction).await {
        log::error!("Usage limit check failed: {e}");
        return Err(e);
    }
    log::error!("Usage limit check passed");

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

    log::error!("Signing and sending transaction");
    let (signature, signed_transaction) =
        match resolved_transaction.sign_and_send_transaction(&signer, rpc_client).await {
            Ok((sig, tx)) => {
                log::error!("Transaction signed and sent successfully: signature={sig}");
                (sig, tx)
            }
            Err(e) => {
                log::error!("Sign and send transaction failed: {e}");
                return Err(e);
            }
        };

    log::error!(
        "RPC Method: signAndSendTransaction - Success: signature={}, signer_pubkey={}",
        signature,
        signer.solana_pubkey()
    );

    Ok(SignAndSendTransactionResponse {
        signature,
        signed_transaction,
        signer_pubkey: signer.solana_pubkey().to_string(),
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
