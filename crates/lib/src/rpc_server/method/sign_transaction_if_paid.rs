use crate::{
    state::get_request_signer_with_signer_key,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionIfPaidResponse {
    pub transaction: String,
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_transaction_if_paid(
    rpc_client: &Arc<RpcClient>,
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let transaction_requested = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction_requested, rpc_client).await?;

    let (transaction, signed_transaction) = resolved_transaction
        .sign_transaction_if_paid(&signer, rpc_client)
        .await
        .map_err(|e| KoraError::TokenOperationError(e.to_string()))?;

    Ok(SignTransactionIfPaidResponse {
        transaction: TransactionUtil::encode_versioned_transaction(&transaction),
        signed_transaction,
        signer_pubkey: signer.solana_pubkey().to_string(),
    })
}
