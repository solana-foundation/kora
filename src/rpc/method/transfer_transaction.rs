use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, message::Message, pubkey::Pubkey, system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;
use std::{str::FromStr, sync::Arc};

use crate::common::{
    account::get_or_create_token_account,
    config::ValidationConfig,
    get_signer,
    transaction::uncompile_instructions,
    validation::{TransactionValidator, ValidationMode},
    KoraError, Signer as _, SOL_MINT,
};

#[derive(Debug, Deserialize)]
pub struct TransferTransactionRequest {
    pub amount: u64,
    pub token: String,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Serialize)]
pub struct TransferTransactionResponse {
    pub transaction: String,
}

pub async fn transfer_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    // Parse addresses
    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::InvalidTransaction(format!("Invalid source address: {}", e)))?;
    let destination = Pubkey::from_str(&request.destination).map_err(|e| {
        KoraError::InvalidTransaction(format!("Invalid destination address: {}", e))
    })?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token address: {}", e)))?;

    validator.validate_token_mint(&token_mint)?;

    let mut instructions: Vec<Instruction> = Vec::new();

    // Check if token is SOL or SPL token
    if request.token == SOL_MINT {
        // Handle SOL transfer
        let transfer_ix = system_instruction::transfer(&source, &destination, request.amount);
        instructions.push(transfer_ix);
    } else {
        // Handle SPL token transfer
        // Get or create destination token account
        let (dest_ata, tx) = get_or_create_token_account(rpc_client, &destination, &token_mint)
            .await
            .map_err(|e| {
                KoraError::InvalidTransaction(format!(
                    "Failed to get or create token account: {}",
                    e
                ))
            })?;

        if let Some(tx) = tx {
            instructions
                .extend(uncompile_instructions(&tx.message.instructions, &tx.message.account_keys));
        }

        // Get ATA for source
        let source_ata = get_associated_token_address(&source, &token_mint);

        // Add transfer instruction
        let transfer_ix = token_instruction::transfer(
            &spl_token::id(),
            &source_ata,
            &dest_ata,
            &source,
            &[&signer.solana_pubkey()],
            request.amount,
        )
        .map_err(|e| {
            KoraError::InvalidTransaction(format!("Failed to create transfer instruction: {}", e))
        })?;

        instructions.push(transfer_ix);
    }

    // Get recent blockhash
    let blockhash =
        rpc_client.get_latest_blockhash().await.map_err(|e| KoraError::Rpc(e.to_string()))?;

    // Create transaction with fee payer
    let message =
        Message::new_with_blockhash(&instructions, Some(&signer.solana_pubkey()), &blockhash);
    let mut transaction = Transaction::new_unsigned(message);

    // Add fee payer signature
    let signature = signer
        .partial_sign(&transaction.message_data())
        .map_err(|e| KoraError::SigningError(format!("Failed to sign transaction: {}", e)))?;
    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;
    transaction.signatures = vec![solana_sdk::signature::Signature::from(sig_bytes)];

    // Validate transaction
    validator.validate_transaction(&transaction, ValidationMode::SignAndSend)?;

    // Serialize and encode transaction
    let serialized = bincode::serialize(&transaction).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to serialize transaction: {}", e))
    })?;

    let encoded = bs58::encode(serialized).into_string();

    Ok(TransferTransactionResponse { transaction: encoded })
}
