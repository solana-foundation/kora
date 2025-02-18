use std::sync::Arc;
use utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

use kora_lib::{
    config::ValidationConfig,
    transaction::{
        decode_b58_transaction, sign_and_send_transaction as lib_sign_and_send_transaction,
    },
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
    let transaction = decode_b58_transaction(&request.transaction)?;
    let (signature, signed_transaction) =
        lib_sign_and_send_transaction(rpc_client, validation, transaction).await?;

    Ok(SignAndSendTransactionResponse { signature, signed_transaction })
}
