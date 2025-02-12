use kora_lib::{
    config::ValidationConfig,
    transaction::{
        decode_b58_transaction, sign_transaction_if_paid as lib_sign_transaction_if_paid,
    },
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
    pub margin: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SignTransactionIfPaidResponse {
    pub signature: String,
    pub signed_transaction: String,
}

pub async fn sign_transaction_if_paid(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let transaction = decode_b58_transaction(&request.transaction)?;
    let (transaction, signed_transaction) =
        lib_sign_transaction_if_paid(rpc_client, validation, transaction, request.margin).await?;

    Ok(SignTransactionIfPaidResponse {
        signature: transaction.signatures[0].to_string(),
        signed_transaction,
    })
}
