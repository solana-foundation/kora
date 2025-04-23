use kora_lib::{
    config::ValidationConfig,
    transaction::{
        decode_b64_transaction, decode_b64_transaction_with_version, encode_b64_transaction,
        encode_b64_transaction_with_version, sign_transaction as lib_sign_transaction,
        sign_versioned_transaction as lib_sign_versioned_transaction,
    },
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
    // Use a unified approach for both transaction types
    let result = match try_sign_versioned_tx(rpc_client, validation, &request.transaction).await {
        Ok(response) => Ok(response),
        Err(_) => try_sign_regular_tx(rpc_client, validation, &request.transaction).await,
    };

    result
}

async fn try_sign_versioned_tx(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
) -> Result<SignTransactionResponse, KoraError> {
    // Decode and sign versioned transaction
    let versioned_tx = decode_b64_transaction_with_version(tx_data)?;
    let (signed_tx, _) =
        lib_sign_versioned_transaction(rpc_client, validation, versioned_tx).await?;

    let encoded = encode_b64_transaction_with_version(&signed_tx)?;

    Ok(SignTransactionResponse {
        signature: signed_tx.signatures[0].to_string(),
        signed_transaction: encoded,
    })
}

async fn try_sign_regular_tx(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
) -> Result<SignTransactionResponse, KoraError> {
    // Decode and sign regular transaction
    let regular_tx = decode_b64_transaction(tx_data)?;
    let (signed_tx, _) = lib_sign_transaction(rpc_client, validation, regular_tx).await?;

    let encoded = encode_b64_transaction(&signed_tx)?;

    Ok(SignTransactionResponse {
        signature: signed_tx.signatures[0].to_string(),
        signed_transaction: encoded,
    })
}
