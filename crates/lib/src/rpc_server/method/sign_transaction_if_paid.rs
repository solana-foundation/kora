use crate::{
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
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
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let transaction_requested = TransactionUtil::decode_b64_transaction(&request.transaction)?;

    let mut resolved_transaction =
        VersionedTransactionResolved::from_transaction(&transaction_requested, rpc_client).await?;

    let (transaction, signed_transaction) = resolved_transaction
        .sign_transaction_if_paid(rpc_client)
        .await
        .map_err(|e| KoraError::TokenOperationError(e.to_string()))?;

    Ok(SignTransactionIfPaidResponse {
        transaction: transaction.encode_b64_transaction()?,
        signed_transaction,
    })
}
