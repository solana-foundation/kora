use kora_lib::{
    config::ValidationConfig,
    transaction::{
        sign_transaction as lib_sign_transaction,
        sign_versioned_transaction as lib_sign_versioned_transaction,
    },
    types::TransactionEncoding,
    KoraError,
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

    // Use a unified approach for both transaction types
    let result =
        match try_sign_versioned_tx(rpc_client, validation, &request.transaction, encoding.clone())
            .await
        {
            Ok(response) => Ok(response),
            Err(_) => {
                try_sign_regular_tx(rpc_client, validation, &request.transaction, encoding).await
            }
        };

    result
}

async fn try_sign_versioned_tx(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
    encoding: TransactionEncoding,
) -> Result<SignTransactionResponse, KoraError> {
    // Decode and sign versioned transaction
    let versioned_tx = encoding.decode_versioned(tx_data)?;
    let (signed_tx, _) =
        lib_sign_versioned_transaction(rpc_client, validation, versioned_tx).await?;

    let encoded = encoding.encode_versioned(&signed_tx)?;

    Ok(SignTransactionResponse {
        signature: signed_tx.signatures[0].to_string(),
        signed_transaction: encoded,
        encoding,
    })
}

async fn try_sign_regular_tx(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    tx_data: &str,
    encoding: TransactionEncoding,
) -> Result<SignTransactionResponse, KoraError> {
    // Decode and sign regular transaction
    let regular_tx = encoding.decode_transaction(tx_data)?;
    let (signed_tx, _) = lib_sign_transaction(rpc_client, validation, regular_tx).await?;

    let encoded = encoding.encode_transaction(&signed_tx)?;

    Ok(SignTransactionResponse {
        signature: signed_tx.signatures[0].to_string(),
        signed_transaction: encoded,
        encoding,
    })
}
