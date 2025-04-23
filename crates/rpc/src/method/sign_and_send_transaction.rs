use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use utoipa::ToSchema;

use kora_lib::{
    config::ValidationConfig,
    transaction::{
        decode_b64_transaction, decode_b64_transaction_with_version,
        sign_and_send_transaction as lib_sign_and_send_transaction,
        sign_and_send_transaction_with_version as lib_sign_and_send_versioned_transaction,
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
    // Use a unified approach for both transaction types
    match try_sign_send_versioned(rpc_client, validation, &request.transaction).await {
        Ok(response) => Ok(response),
        Err(_) => try_sign_send_regular(rpc_client, validation, &request.transaction).await,
    }
}

async fn try_sign_send_versioned(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    let versioned_tx = decode_b64_transaction_with_version(tx_data)?;
    let (signature, signed_transaction) =
        lib_sign_and_send_versioned_transaction(rpc_client, validation, versioned_tx).await?;

    Ok(SignAndSendTransactionResponse { signature, signed_transaction })
}

async fn try_sign_send_regular(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
) -> Result<SignAndSendTransactionResponse, KoraError> {
    let regular_tx = decode_b64_transaction(tx_data)?;
    let (signature, signed_transaction) =
        lib_sign_and_send_transaction(rpc_client, validation, regular_tx).await?;

    Ok(SignAndSendTransactionResponse { signature, signed_transaction })
}
