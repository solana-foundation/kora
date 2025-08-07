use kora_lib::{
    config::ValidationConfig,
    transaction::{TransactionUtil, VersionedTransactionUtilExt},
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionRequest {
    pub transaction: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionResponse {
    pub signature: String,
    pub signed_transaction: String,
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionRequest,
) -> Result<SignTransactionResponse, KoraError> {
    let transaction = TransactionUtil::decode_b64_transaction(&request.transaction)?;
    let _signed_transaction = transaction.sign_transaction(rpc_client, validation).await?;

    let encoded = transaction.encode_b64_transaction()?;

    Ok(SignTransactionResponse {
        signature: transaction.signatures[0].to_string(),
        signed_transaction: encoded,
    })
}
