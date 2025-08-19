use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    state::get_request_signer,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    KoraError,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignAndSendTransactionRequest {
    pub transaction: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignAndSendTransactionResponse {
    pub signature: String,
    pub signed_transaction: String,
}

pub async fn sign_and_send_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignAndSendTransactionRequest,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let signer = get_request_signer()?;

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction, rpc_client).await?;

    let (signature, signed_transaction) =
        resolved_transaction.sign_and_send_transaction(&signer, rpc_client).await?;

    Ok(SignAndSendTransactionResponse { signature, signed_transaction })
}
