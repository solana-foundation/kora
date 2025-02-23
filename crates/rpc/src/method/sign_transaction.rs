use kora_lib::{
    config::ValidationConfig, transaction::sign_transaction as lib_sign_transaction,
    types::TransactionEncoding, KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionRequest {
    pub transaction: String,
    #[serde(default)]
    pub encoding: Option<TransactionEncoding>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionResponse {
    pub signature: String,
    pub signed_transaction: String,
    pub encoding: TransactionEncoding,
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionRequest,
) -> Result<SignTransactionResponse, KoraError> {
    let encoding = request.encoding.unwrap_or_default();
    let transaction = encoding.decode_transaction(&request.transaction)?;
    let (transaction, _signed_transaction) =
        lib_sign_transaction(rpc_client, validation, transaction).await?;

    let encoded = encoding.encode_transaction(&transaction)?;

    Ok(SignTransactionResponse {
        signature: transaction.signatures[0].to_string(),
        signed_transaction: encoded,
        encoding,
    })
}
