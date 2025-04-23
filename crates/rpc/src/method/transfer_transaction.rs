use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, message::Message, pubkey::Pubkey, system_instruction,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc};
use utoipa::ToSchema;

use kora_lib::{
    config::ValidationConfig,
    constant::NATIVE_SOL,
    get_signer,
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::{encode_b64_message, encode_b64_transaction, validator::TransactionValidator},
    KoraError, Signer as _,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferTransactionRequest {
    pub amount: u64,
    pub token: String,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TransferTransactionResponse {
    pub transaction: String,
    pub message: String,
    pub blockhash: String,
}

pub async fn transfer_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_signer()?;
    let fee_payer = signer.solana_pubkey();

    let validator = TransactionValidator::new(fee_payer, validation)?;

    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::ValidationError(format!("Invalid source address: {}", e)))?;
    let destination = Pubkey::from_str(&request.destination)
        .map_err(|e| KoraError::ValidationError(format!("Invalid destination address: {}", e)))?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::ValidationError(format!("Invalid token address: {}", e)))?;

    // manually check disallowed account because we're creating the message
    if validator.is_disallowed_account(&source) {
        return Err(KoraError::InvalidTransaction(format!(
            "Source account {} is disallowed",
            source
        )));
    }

    if validator.is_disallowed_account(&destination) {
        return Err(KoraError::InvalidTransaction(format!(
            "Destination account {} is disallowed",
            destination
        )));
    }

    let mut instructions = vec![];

    // Handle native SOL transfers
    if request.token == NATIVE_SOL {
        instructions.push(system_instruction::transfer(&source, &destination, request.amount));
    } else {
        // Handle wrapped SOL and other SPL tokens
        validator.validate_token_mint(&token_mint, rpc_client).await?;

        let mint_account = rpc_client
            .get_account(&token_mint)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        let token_program = TokenProgram::new(TokenType::Spl);
        let decimals = token_program
            .get_mint_decimals(&mint_account.data)
            .map_err(|_| KoraError::ValidationError("Invalid mint account data".to_string()))?;

        let source_ata = token_program.get_associated_token_address(&source, &token_mint);
        let dest_ata = token_program.get_associated_token_address(&destination, &token_mint);

        let _ = rpc_client
            .get_account(&source_ata)
            .await
            .map_err(|_| KoraError::AccountNotFound(source_ata.to_string()))?;

        if rpc_client.get_account(&dest_ata).await.is_err() {
            instructions.push(token_program.create_associated_token_account_instruction(
                &fee_payer,
                &destination,
                &token_mint,
            ));
        }

        instructions.push(
            token_program
                .create_transfer_checked_instruction(
                    &source_ata,
                    &token_mint,
                    &dest_ata,
                    &source,
                    request.amount,
                    decimals,
                )
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to create transfer instruction: {}",
                        e
                    ))
                })?,
        );
    }

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    let message = Message::new_with_blockhash(&instructions, Some(&fee_payer), &blockhash.0);
    let mut transaction = Transaction::new_unsigned(message);

    // validate transaction before signing
    validator.validate_transaction(&transaction)?;

    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    let encoded = encode_b64_transaction(&transaction)?;
    let message_encoded = encode_b64_message(&transaction.message)?;

    Ok(TransferTransactionResponse {
        transaction: encoded,
        message: message_encoded,
        blockhash: blockhash.0.to_string(),
    })
}
