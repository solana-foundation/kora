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
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction, rpc_client).await?;

    let (signature, signed_transaction) =
        resolved_transaction.sign_and_send_transaction(&signer, rpc_client).await?;

    Ok(SignAndSendTransactionResponse {
        signature,
        signed_transaction,
        signer_pubkey: signer.solana_pubkey().to_string(),
    })
}
