use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;

use crate::common::{
    config::ValidationConfig,
    get_signer,
    transaction::decode_b58_transaction,
    validation::{TransactionValidator, ValidationMode},
    KoraError,
};

use super::sign_transaction::{sign_transaction, SignTransactionRequest};

#[derive(Debug, Deserialize)]
pub struct SignAndSendTransactionRequest {
    pub transaction: String,
}

#[derive(Debug, Serialize)]
pub struct SignAndSendTransactionResult {
    pub signature: String,
    pub signed_transaction: String,
}

pub async fn sign_and_send(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignAndSendTransactionRequest,
) -> Result<SignAndSendTransactionResult, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    let original_transaction = decode_b58_transaction(&request.transaction)?;

    // Create validator
    let validator = TransactionValidator::new(signer.solana_pubkey(), &validation)?;

    // Validate transaction with SignAndSend mode
    validator.validate_transaction(&original_transaction, ValidationMode::SignAndSend)?;

    let sign_result = match sign_transaction(
        rpc_client,
        validation,
        SignTransactionRequest { transaction: request.transaction },
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return Err(e),
    };

    // Decode the signed transaction from base58
    let signed_transaction_bytes = bs58::decode(&sign_result.signed_transaction)
        .into_vec()
        .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;

    // Deserialize into a Transaction object
    let signed_transaction: solana_sdk::transaction::Transaction =
        bincode::deserialize(&signed_transaction_bytes)
            .map_err(|e| KoraError::InvalidTransaction(e.to_string()))?;

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&signed_transaction)
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    Ok(SignAndSendTransactionResult {
        signature: signature.to_string(),
        signed_transaction: sign_result.signed_transaction,
    })
}
