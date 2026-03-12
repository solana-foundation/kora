use crate::{
    rpc_server::middleware_utils::default_sig_verify,
    state::{get_config, get_request_signer_with_signer_key},
    swap::SwapForGasProcessor,
    transaction::TransactionUtil,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
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

    let config = get_config()?;
    let sig_verify = request.sig_verify || config.kora.force_sig_verify;
    let signed_transaction = SwapForGasProcessor::validate_transaction_for_send(
        &transaction,
        &signer_pubkey,
        &signer,
        config,
        rpc_client,
        sig_verify,
        request.user_id.as_deref(),
    )
    .await?;

    let signature = rpc_client
        .send_and_confirm_transaction(&signed_transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    let signed_transaction = TransactionUtil::encode_versioned_transaction(&signed_transaction)?;

    Ok(SignAndSendSwapForGasResponse {
        signed_transaction,
        signer_pubkey: signer_pubkey.to_string(),
        signature: signature.to_string(),
    })
}
