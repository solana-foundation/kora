use crate::common::{
    config::ValidationConfig,
    get_signer,
    transaction::{decode_b58_transaction, uncompile_instructions},
    validation::TransactionValidator,
    KoraError, Signer as _,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, message::Message,
    transaction::Transaction,
};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignTransactionRequest {
    pub transaction: String,
}

#[derive(Debug, Serialize)]
pub struct SignTransactionResult {
    pub signature: String,
    pub signed_transaction: String,
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionRequest,
) -> Result<SignTransactionResult, KoraError> {
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

    let signature = signer.partial_sign_solana(&transaction.message_data())?;

    transaction.signatures[0] = signature;

    let serialized = bincode::serialize(&transaction).map_err(|e| KoraError::InvalidTransaction(format!("Failed to serialize transaction: {}", e)))?;
    let encoded = bs58::encode(serialized).into_string();
    
    Ok(SignTransactionResult { 
        signature: signature.to_string(), 
        signed_transaction: encoded 
    })
}