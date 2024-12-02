use crate::common::{get_signer, transaction::decode_b58_transaction, KoraError, Signer as _};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::Signature,
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

fn compile_instructions(
    instructions: &[CompiledInstruction],
    account_keys: &[Pubkey],
) -> Vec<Instruction> {
    instructions
        .iter()
        .map(|ix| {
            let program_id = account_keys[ix.program_id_index as usize];
            let accounts = ix
                .accounts
                .iter()
                .map(|idx| AccountMeta {
                    pubkey: account_keys[*idx as usize],
                    is_signer: false, // We'll set this based on original transaction
                    is_writable: true, // We'll set this based on original transaction
                })
                .collect();

            Instruction { program_id, accounts, data: ix.data.clone() }
        })
        .collect()
}

pub async fn sign_transaction(
    rpc_client: &Arc<RpcClient>,
    request: SignTransactionRequest,
) -> Result<SignTransactionResult, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    log::info!("Signing transaction: {}", request.transaction);

    let original_transaction = decode_b58_transaction(&request.transaction)?;

    let blockhash =
        rpc_client.get_latest_blockhash().await.map_err(|e| KoraError::Rpc(e.to_string()))?;

    let compiled_instructions = compile_instructions(
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
