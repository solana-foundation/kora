use kora_lib::{
    config::ValidationConfig,
    transaction::{sign_transaction_if_paid as lib_sign_transaction_if_paid, TokenPriceInfo},
    types::TransactionEncoding,
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
    #[serde(default)]
    pub encoding: Option<TransactionEncoding>,
    pub margin: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignTransactionIfPaidResponse {
    pub signature: String,
    pub signed_transaction: String,
    pub encoding: TransactionEncoding,
}

pub async fn sign_transaction_if_paid(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let encoding = request.encoding.unwrap_or_default();
    let transaction = encoding.decode_transaction(&request.transaction)?;
    let (transaction, signed_transaction) = lib_sign_transaction_if_paid(
        rpc_client,
        validation,
        transaction,
        request.margin,
        request.token_price_info,
    )
    .await?;

    let encoded = encoding.encode_transaction(&transaction)?;

    Ok(SignTransactionIfPaidResponse {
        signature: transaction.signatures[0].to_string(),
        signed_transaction: encoded,
        encoding,
    })
}
