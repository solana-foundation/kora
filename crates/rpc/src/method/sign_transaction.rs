use kora_lib::{
    config::ValidationConfig, get_signer, transaction::decode_b58_transaction,
    validation::TransactionValidator, KoraError, Signer as _,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
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
    let signer = get_signer()?;

    let original_transaction = decode_b58_transaction(&request.transaction)?;
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;
    validator.validate_transaction(&original_transaction)?;
    validator.validate_disallowed_accounts(&original_transaction.message)?;

    let mut transaction = original_transaction;

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    transaction.message.recent_blockhash = blockhash.0;

    let estimated_fee = rpc_client.get_fee_for_message(&transaction.message).await?;
    validator.validate_lamport_fee(estimated_fee)?;

    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    let serialized = bincode::serialize(&transaction)?;
    let encoded = bs58::encode(serialized).into_string();

    Ok(SignTransactionResult { signature: signature.to_string(), signed_transaction: encoded })
}
