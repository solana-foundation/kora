use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;

use crate::common::{get_signer, transaction::decode_b58_transaction, KoraError, Signer as _};

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
    request: SignTransactionRequest,
) -> Result<SignTransactionResult, KoraError> {
    // TODO: Validate tx

    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    log::info!("Signing transaction: {}", request.transaction);

    // Decode the transaction from base64
    let mut transaction = decode_b58_transaction(&request.transaction)?;

    // Sign the transaction as fee payer
    let message = transaction.message.clone();
    let signature = signer
        .partial_sign(message.serialize().as_slice())
        .map_err(|e| KoraError::SigningError(format!("Failed to sign transaction: {}", e)))?;

    // Replace the first signature (fee payer) with our signature
    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;
    let sig = Signature::from(sig_bytes);
    transaction.signatures[0] = sig;

    // Serialize the signed transaction back to bytes and encode as base64
    let signed_transaction = bincode::serialize(&transaction).map_err(|e| {
        KoraError::InternalServerError(format!("Failed to serialize transaction: {}", e))
    })?;

    Ok(SignTransactionResult {
        signature: sig.to_string(),
        signed_transaction: bs58::encode(signed_transaction).into_string(),
    })
}
