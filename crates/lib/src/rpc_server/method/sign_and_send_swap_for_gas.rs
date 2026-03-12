use crate::{
    rpc_server::middleware_utils::default_sig_verify,
    state::{get_config, get_request_signer_with_signer_key},
    transaction::{TransactionUtil, VersionedTransactionResolved},
    usage_limit::UsageTracker,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignAndSendSwapForGasRequest {
    pub transaction: String,
    /// Optional signer key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// Optional user ID for usage tracking (required when pricing is free and usage tracking is enabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignAndSendSwapForGasResponse {
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
    /// Transaction signature
    pub signature: String,
}

pub async fn sign_and_send_swap_for_gas(
    rpc_client: &Arc<RpcClient>,
    request: SignAndSendSwapForGasRequest,
) -> Result<SignAndSendSwapForGasResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let signer_pubkey = signer.pubkey();

    let account_keys = transaction.message.static_account_keys();
    let required_signers = transaction.message.header().num_required_signatures as usize;

    let signer_position =
        account_keys.iter().position(|key| key == &signer_pubkey).ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Signer {signer_pubkey} not found in transaction account keys. \
                 Pass signer_key returned by signSwapForGas."
            ))
        })?;

    if signer_position >= required_signers {
        return Err(KoraError::InvalidTransaction(format!(
            "Signer {signer_pubkey} is not a required signer for this transaction"
        )));
    }

    let signer_signature = transaction.signatures[signer_position];
    if signer_signature == Signature::default() {
        return Err(KoraError::ValidationError(
            "Missing Kora signer signature. Call signSwapForGas first and pass signer_key in signAndSendSwapForGas."
                .to_string(),
        ));
    }

    let message_bytes = transaction.message.serialize();
    if !signer_signature.verify(signer_pubkey.as_ref(), &message_bytes) {
        return Err(KoraError::InvalidTransaction(
            "Invalid Kora signer signature for the provided transaction".to_string(),
        ));
    }

    let config = get_config()?;
    let sig_verify = request.sig_verify || config.kora.force_sig_verify;
    let mut resolved_transaction = VersionedTransactionResolved::from_transaction(
        &transaction,
        config,
        rpc_client,
        sig_verify,
    )
    .await?;

    UsageTracker::check_transaction_usage_limit(
        config,
        &mut resolved_transaction,
        request.user_id.as_deref(),
        &signer_pubkey,
        rpc_client,
    )
    .await?;

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    let signed_transaction = TransactionUtil::encode_versioned_transaction(&transaction)?;

    Ok(SignAndSendSwapForGasResponse {
        signed_transaction,
        signer_pubkey: signer_pubkey.to_string(),
        signature: signature.to_string(),
    })
}
