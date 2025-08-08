use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    config::ValidationConfig,
    transaction::{TransactionUtil, VersionedTransactionUtilExt},
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
    validation: &ValidationConfig,
    request: SignAndSendTransactionRequest,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let (signature, signed_transaction) =
        transaction.sign_and_send_transaction(rpc_client, validation).await?;

    Ok(SignAndSendTransactionResponse { signature, signed_transaction })
}
