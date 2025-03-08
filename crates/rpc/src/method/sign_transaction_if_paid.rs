use std::sync::Arc;

use kora_lib::{
    config::ValidationConfig,
    transaction::{
        encode_transaction_b58, sign_transaction_if_paid as lib_sign_transaction_if_paid,
    },
    types::TransactionEncoding,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: Transaction,
    #[serde(default)]
    pub encoding: Option<TransactionEncoding>,
    pub margin: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionIfPaidResponse {
    pub transaction: String,
    pub signed_transaction: String,
}

pub async fn sign_transaction_if_paid(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let (transaction, signed_transaction) =
        lib_sign_transaction_if_paid(rpc_client, validation, request.transaction, request.margin)
            .await?;

    Ok(SignTransactionIfPaidResponse {
        transaction: encode_transaction_b58(&transaction)?,
        signed_transaction,
    })
}
