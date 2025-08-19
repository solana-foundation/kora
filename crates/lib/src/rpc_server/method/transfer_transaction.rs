use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_message::Message;
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey};
use solana_system_interface::instruction::transfer;
use std::{str::FromStr, sync::Arc};
use utoipa::ToSchema;

use crate::{
    constant::NATIVE_SOL,
    get_signer,
    transaction::{
        TransactionUtil, VersionedMessageExt, VersionedTransactionOps, VersionedTransactionResolved,
    },
    validator::transaction_validator::TransactionValidator,
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
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_signer()?;
    let fee_payer = signer.solana_pubkey();

    let validator = TransactionValidator::new(fee_payer)?;

    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::ValidationError(format!("Invalid source address: {e}")))?;
    let destination = Pubkey::from_str(&request.destination)
        .map_err(|e| KoraError::ValidationError(format!("Invalid destination address: {e}")))?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::ValidationError(format!("Invalid token address: {e}")))?;

    // manually check disallowed account because we're creating the message
    if validator.is_disallowed_account(&source) {
        return Err(KoraError::InvalidTransaction(format!(
            "Source account {source} is disallowed"
        )));
    }

    if validator.is_disallowed_account(&destination) {
        return Err(KoraError::InvalidTransaction(format!(
            "Destination account {destination} is disallowed"
        )));
    }

    let mut instructions = vec![];

    // Handle native SOL transfers
    if request.token == NATIVE_SOL {
        instructions.push(transfer(&source, &destination, request.amount));
    } else {
        // Handle wrapped SOL and other SPL tokens
        let token_mint = validator.fetch_and_validate_token_mint(&token_mint, rpc_client).await?;
        let token_program = token_mint.get_token_program();
        let decimals = token_mint.decimals();

        let source_ata = token_program.get_associated_token_address(&source, &token_mint.address());
        let dest_ata =
            token_program.get_associated_token_address(&destination, &token_mint.address());

        let _ = rpc_client
            .get_account(&source_ata)
            .await
            .map_err(|_| KoraError::AccountNotFound(source_ata.to_string()))?;

        if rpc_client.get_account(&dest_ata).await.is_err() {
            instructions.push(token_program.create_associated_token_account_instruction(
                &fee_payer,
                &destination,
                &token_mint.address(),
            ));
        }

        instructions.push(
            token_program
                .create_transfer_checked_instruction(
                    &source_ata,
                    &token_mint.address(),
                    &dest_ata,
                    &source,
                    request.amount,
                    decimals,
                )
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to create transfer instruction: {e}"
                    ))
                })?,
        );
    }

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&fee_payer),
        &blockhash.0,
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    let mut resolved_transaction =
        VersionedTransactionResolved::from_kora_built_transaction(&transaction);

    // validate transaction before signing
    validator.validate_transaction(&mut resolved_transaction).await?;

    // Find the fee payer position in the account keys
    let fee_payer_position = resolved_transaction.find_signer_position(&fee_payer)?;

    let signature = signer.sign_solana(&resolved_transaction).await?;

    resolved_transaction.transaction.signatures[fee_payer_position] = signature;

    let encoded = resolved_transaction.encode_b64_transaction()?;
    let message_encoded = transaction.message.encode_b64_message()?;

    Ok(TransferTransactionResponse {
        transaction: encoded,
        message: message_encoded,
        blockhash: blockhash.0.to_string(),
    })
}
