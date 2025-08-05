use kora_lib::{
    config::ValidationConfig,
    transaction::{
        decode_b64_transaction, encode_b64_transaction,
        sign_transaction_if_paid as lib_sign_transaction_if_paid, VersionedTransactionResolved,
    },
    KoraError,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
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
    let transaction_requested = decode_b64_transaction(&request.transaction)?;

    let mut resolved_transaction = VersionedTransactionResolved::new(&transaction_requested);
    resolved_transaction.resolve_addresses(rpc_client).await?;

    let (transaction, signed_transaction) =
        lib_sign_transaction_if_paid(rpc_client, validation, &resolved_transaction)
            .await
            .map_err(|e| KoraError::TokenOperationError(e.to_string()))?;

    Ok(SignTransactionIfPaidResponse {
        transaction: encode_b64_transaction(&transaction)?,
        signed_transaction,
    })
}
