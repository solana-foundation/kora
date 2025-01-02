use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

use crate::common::{
    config::ValidationConfig, get_signer, transaction::decode_b58_transaction,
    validation::TransactionValidator, KoraError, Signer as _,
};

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

    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    validator.validate_transaction(&original_transaction)?;

    let mut transaction = original_transaction;

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    transaction.message.recent_blockhash = blockhash.0;

    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    let serialized = bincode::serialize(&transaction).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to serialize transaction: {}", e))
    })?;
    let encoded = bs58::encode(serialized).into_string();

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    Ok(SignAndSendTransactionResult {
        signature: signature.to_string(),
        signed_transaction: encoded,
    })
}
