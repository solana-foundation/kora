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
pub struct SignTransactionRequest {
    pub transaction: String,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionResponse {
    pub signature: String,
    pub signed_transaction: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignTransactionRequest,
) -> Result<SignTransactionResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction, rpc_client).await?;

    let (signed_transaction, _) =
        resolved_transaction.sign_transaction(&signer, rpc_client).await?;

    let encoded = TransactionUtil::encode_versioned_transaction(&signed_transaction);

    Ok(SignTransactionResponse {
        signature: transaction.signatures[0].to_string(),
        signed_transaction: encoded,
        signer_pubkey: signer.solana_pubkey().to_string(),
    })
}
