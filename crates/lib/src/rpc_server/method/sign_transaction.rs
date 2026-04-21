use crate::{
    rpc_server::middleware_utils::default_sig_verify,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    usage_limit::UsageTracker,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

#[cfg(not(test))]
use crate::state::{get_config, select_request_signer_with_signer_key};

#[cfg(test)]
use crate::state::select_request_signer_with_signer_key;
#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Request payload for signing a transaction.
///
/// This endpoint accepts a base64-encoded Solana transaction, validates it against
/// the configured fee payer policies, and if successful, signs it using Kora's
/// configured signer policy
/// but not broadcasted to the network.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionRequest {
    /// Base64-encoded Solana transaction
    pub transaction: String,
    /// Optional public key of the signer to ensure consistency
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// Optional user ID for usage tracking (required when pricing is Free and usage tracking is enabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Response payload containing the signed transaction.
#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionResponse {
    /// Base64-encoded transaction signed by the Kora fee payer
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignTransactionRequest,
) -> Result<SignTransactionResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;

    let config = &get_config()?;

    let signer = select_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let fee_payer = signer.pubkey();

    let sig_verify = request.sig_verify || config.kora.force_sig_verify;
    let mut resolved_transaction = VersionedTransactionResolved::from_transaction(
        &transaction,
        config,
        rpc_client,
        sig_verify,
    )
    .await?;

    // Check usage limit for transaction sender
    UsageTracker::check_transaction_usage_limit(
        config,
        &mut resolved_transaction,
        request.user_id.as_deref(),
        &fee_payer,
        rpc_client,
    )
    .await?;

    let (signed_transaction, _) =
        resolved_transaction.sign_transaction(config, &signer, rpc_client, false).await?;

    let encoded = TransactionUtil::encode_versioned_transaction(&signed_transaction)?;

    Ok(SignTransactionResponse {
        signed_transaction: encoded,
        signer_pubkey: signer.pubkey().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::{get_signer_pool, update_config, update_signer_pool},
        tests::{
            common::{
                create_probe_eligible_test_pool, setup_or_get_test_signer,
                setup_or_get_test_usage_limiter, RpcMockBuilder,
            },
            config_mock::ConfigMockBuilder,
            transaction_mock::create_mock_encoded_transaction,
        },
        transaction::TransactionUtil,
    };
    use serial_test::serial;
    use solana_compute_budget_interface::ComputeBudgetInstruction;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    fn create_compute_budget_only_encoded_transaction() -> String {
        let message = VersionedMessage::Legacy(Message::new(
            &[ComputeBudgetInstruction::set_compute_unit_limit(200_000)],
            Some(&Pubkey::new_unique()),
        ));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        TransactionUtil::encode_versioned_transaction(&transaction).unwrap()
    }

    #[tokio::test]
    async fn test_sign_transaction_decode_error() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignTransactionRequest {
            transaction: "invalid_base64!@#$".to_string(),
            signer_key: None,
            sig_verify: true,
            user_id: None,
        };

        let result = sign_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with decode error");
    }

    #[tokio::test]
    async fn test_sign_transaction_invalid_signer_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = SignTransactionRequest {
            transaction: create_mock_encoded_transaction(),
            signer_key: Some("invalid_pubkey".to_string()),
            sig_verify: true,
            user_id: None,
        };

        let result = sign_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::ValidationError(_)), "Should return ValidationError");
    }

    #[tokio::test]
    #[serial]
    async fn test_sign_transaction_pre_sign_validation_error_does_not_pin_probe_lock() {
        let _config_guard = ConfigMockBuilder::new().build_and_setup();
        update_config(ConfigMockBuilder::new().build()).unwrap();
        let (pool, target_pubkey) = create_probe_eligible_test_pool();
        update_signer_pool(pool).unwrap();

        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client = Arc::new(RpcMockBuilder::new().build());
        let request = SignTransactionRequest {
            transaction: create_compute_budget_only_encoded_transaction(),
            signer_key: Some(target_pubkey.clone()),
            sig_verify: true,
            user_id: None,
        };

        let result = sign_transaction(&rpc_client, request).await;
        assert!(matches!(
            result,
            Err(KoraError::InvalidTransaction(message))
                if message.contains("only ComputeBudget instructions")
        ));

        let target_pubkey = Pubkey::from_str(&target_pubkey).unwrap();
        let pool = get_signer_pool().unwrap();
        assert!(!pool.probe_in_flight(&target_pubkey).unwrap());
        assert!(pool.get_signer_by_pubkey(&target_pubkey.to_string()).is_ok());
    }
}
