use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

use crate::common::KoraError;

use super::sign_transaction::{sign_transaction, SignTransactionRequest};


#[derive(Debug, Deserialize)]
pub struct SignAndSendTransactionRequest {
    pub transaction: String, // Base64 encoded transaction
}

#[derive(Debug, Serialize)]
pub struct SignAndSendTransactionResult {
    pub signature: String,
    pub signed_transaction: String, // Base64 encoded signed transaction
}

pub async fn sign_and_send(rpc_client: &Arc<RpcClient>, request: SignAndSendTransactionRequest) -> Result<SignAndSendTransactionResult, KoraError> {
    let sign_result = match sign_transaction(
        SignTransactionRequest {
            transaction: request.transaction,
        }
    ).await {
        Ok(result) => result,
        Err(e) => return Err(e),
    };

    // Decode the signed transaction from base58
    let signed_transaction_bytes = bs58::decode(&sign_result.signed_transaction)
        .into_vec()
        .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;

    // Deserialize into a Transaction object
    let signed_transaction: solana_sdk::transaction::Transaction = bincode::deserialize(&signed_transaction_bytes)
        .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;

    // Send and confirm transaction
    let signature = rpc_client.send_and_confirm_transaction(&signed_transaction)
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    Ok(SignAndSendTransactionResult {
        signature: signature.to_string(),
        signed_transaction: sign_result.signed_transaction,
    })
}
