use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, message::Message, program_pack::Pack, pubkey::Pubkey,
    signature::Keypair, signer::Signer, system_instruction, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::{create_associated_token_account, create_associated_token_account_idempotent},
};
use spl_token::{instruction as token_instruction, state::Mint};
use std::{str::FromStr, sync::Arc};

use crate::common::{
    config::ValidationConfig, get_signer, validation::TransactionValidator, KoraError, Signer as _,
    NATIVE_SOL,
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
    pub message: String,
    pub blockhash: String,
}

pub async fn transfer_transaction(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;
    let fee_payer = signer.solana_pubkey();

    let validator = TransactionValidator::new(fee_payer, validation)?;

    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::InvalidTransaction(format!("Invalid source address: {}", e)))?;
    let destination = Pubkey::from_str(&request.destination).map_err(|e| {
        KoraError::InvalidTransaction(format!("Invalid destination address: {}", e))
    })?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token address: {}", e)))?;

    let mut instructions = vec![];

    // Handle native SOL transfers
    if request.token == NATIVE_SOL {
        instructions.push(system_instruction::transfer(&source, &destination, request.amount));
    } else {
        // Handle wrapped SOL and other SPL tokens
        validator.validate_token_mint(&token_mint)?;

        let mint_account = rpc_client.get_account(&token_mint).await.map_err(|_| {
            KoraError::InvalidTransaction("Failed to fetch mint account".to_string())
        })?;

        let mint = Mint::unpack(&mint_account.data)
            .map_err(|_| KoraError::InvalidTransaction("Invalid mint account data".to_string()))?;

        let decimals = mint.decimals;
        let source_ata = get_associated_token_address(&source, &token_mint);
        let dest_ata = get_associated_token_address(&destination, &token_mint);

        let source_ata_account = rpc_client.get_account(&source_ata).await;
        if source_ata_account.is_err() {
            return Err(KoraError::InvalidTransaction("Source ATA account not found".to_string()));
        }

        match rpc_client.get_account(&dest_ata).await {
            Ok(_) => {}
            Err(_) => {
                instructions.push(create_associated_token_account(
                    &fee_payer,
                    &destination,
                    &token_mint,
                    &spl_token::id(),
                ));
            }
        }

        println!("source_ata: {}", dest_ata);

        instructions.push(
            token_instruction::transfer_checked(
                &spl_token::id(),
                &source_ata,
                &token_mint,
                &dest_ata,
                &source,
                &[],
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

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    let message = Message::new_with_blockhash(&instructions, Some(&fee_payer), &blockhash.0);

    let mut transaction = Transaction::new_unsigned(message);

    let signature = signer
        .partial_sign_solana(&transaction.message_data())
        .map_err(|e| KoraError::SigningError(format!("Failed to sign transaction: {}", e)))?;

    transaction.signatures[0] = signature;

    let serialized = bincode::serialize(&transaction).map_err(|e| {
        KoraError::InvalidTransaction(format!("Failed to serialize transaction: {}", e))
    })?;
    let encoded = bs58::encode(serialized).into_string();

    Ok(TransferTransactionResponse {
        transaction: encoded,
        message: bs58::encode(transaction.message.serialize()).into_string(),
        blockhash: blockhash.0.to_string(),
    })
}
