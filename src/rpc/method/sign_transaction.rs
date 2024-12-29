use crate::common::{
    config::ValidationConfig,
    get_signer,
    transaction::{decode_b58_transaction, uncompile_instructions},
    validation::{TransactionValidator, ValidationMode},
    KoraError, Signer as _,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{message::Message, signature::Signature, transaction::Transaction};
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

    log::info!("Signing transaction: {}", request.transaction);

    let original_transaction = decode_b58_transaction(&request.transaction)?;

    // Create validator with config settings
    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Validate transaction with Sign mode
    validator.validate_transaction(&original_transaction, ValidationMode::Sign)?;

    let blockhash =
        rpc_client.get_latest_blockhash().await.map_err(|e| KoraError::Rpc(e.to_string()))?;

    let compiled_instructions = uncompile_instructions(
        &original_transaction.message.instructions,
        &original_transaction.message.account_keys,
    );

    let message = Message::new_with_blockhash(
        &compiled_instructions,
        Some(&signer.solana_pubkey()),
        &blockhash,
    );

    let mut transaction = Transaction::new_unsigned(message);

    let signature = signer
        .partial_sign(&transaction.message_data())
        .map_err(|e| KoraError::SigningError(format!("Failed to sign transaction: {}", e)))?;

    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

    let sig = Signature::from(sig_bytes);
    transaction.signatures = vec![sig];

    let signed_transaction = bincode::serialize(&transaction).map_err(|e| {
        KoraError::InternalServerError(format!("Failed to serialize transaction: {}", e))
    })?;

    Ok(SignTransactionResult {
        signature: sig.to_string(),
        signed_transaction: bs58::encode(signed_transaction).into_string(),
    })
}
