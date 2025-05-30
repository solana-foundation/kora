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
    try_sign_transaction(rpc_client, validation, &request.transaction).await
}

async fn try_sign_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
) -> Result<SignTransactionResponse, KoraError> {
    // Attempt to decode and sign as a versioned transaction
    if let Ok(versioned_tx) = decode_b64_transaction_with_version(tx_data) {
        let (signed_tx, _) =
            lib_sign_versioned_transaction(rpc_client, validation, versioned_tx).await?;
        let encoded = encode_b64_transaction_with_version(&signed_tx)?;

        return Ok(SignTransactionResponse {
            signature: signed_tx.signatures[0].to_string(),
            signed_transaction: encoded,
        });
    }

    // Fallback to decoding and signing as a regular transaction
    let regular_tx = decode_b64_transaction(tx_data)?;
    let (signed_tx, _) = lib_sign_transaction(rpc_client, validation, regular_tx).await?;
    let encoded = encode_b64_transaction(&signed_tx)?;

    Ok(SignTransactionResponse {
        signature: signed_tx.signatures[0].to_string(),
        signed_transaction: encoded,
    })
}
