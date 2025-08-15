use crate::{
    error::KoraError,
    signer::Signer,
    state::{get_config, get_signer},
    token::token::TokenType,
    transaction::TransactionUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use std::{str::FromStr, sync::Arc};

/// Initialize ATAs for all allowed payment tokens for the paymaster
/// This function does not use cache and directly checks on-chain
pub async fn initialize_paymaster_atas(rpc_client: &Arc<RpcClient>) -> Result<(), KoraError> {
    let signer = get_signer()?;

    let config = get_config()?;

    // Determine the payment address
    let payment_address = config.kora.get_payment_address()?;

    println!("Initializing ATAs for payment address: {payment_address}");

    // Parse all allowed SPL paid token mints
    let mut token_mints = Vec::new();
    for token_str in &config.validation.allowed_spl_paid_tokens {
        match Pubkey::from_str(token_str) {
            Ok(mint) => token_mints.push(mint),
            Err(_) => {
                println!("⚠️  Skipping invalid token mint: {token_str}");
                continue;
            }
        }
    }

    if token_mints.is_empty() {
        println!("✓ No SPL payment tokens configured");
        return Ok(());
    }

    let mut atas_to_create = Vec::new();
    let mut instructions = Vec::new();

    // Check each token mint for existing ATA
    for mint in &token_mints {
        let ata = get_associated_token_address(&payment_address, mint);

        match rpc_client.get_account(&ata).await {
            Ok(_) => {
                println!("✓ ATA already exists for token {mint}: {ata}");
            }
            Err(_) => {
                // Fetch mint account to determine if it's SPL or Token2022
                let mint_account = rpc_client.get_account(mint).await.map_err(|e| {
                    KoraError::RpcError(format!("Failed to fetch mint account for {mint}: {e}"))
                })?;

                let token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;

                println!("Creating ATA for token {mint}: {ata}");

                atas_to_create.push((mint, ata));

                let create_ata_ix = create_associated_token_account(
                    &signer.solana_pubkey(),
                    &payment_address,
                    mint,
                    &token_program.program_id(),
                );
                instructions.push(create_ata_ix);
            }
        }
    }

    if instructions.is_empty() {
        println!("✓ All required ATAs already exist");
        return Ok(());
    }

    println!("Creating {} ATAs in a single transaction...", instructions.len());

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to get blockhash: {e}")))?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&signer.solana_pubkey()),
        &blockhash,
    ));

    let mut tx = TransactionUtil::new_unsigned_versioned_transaction(message);
    let signature = signer.sign(&tx).await?;

    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

    let sig = Signature::from(sig_bytes);
    tx.signatures = vec![sig];

    match rpc_client.send_and_confirm_transaction(&tx).await {
        Ok(signature) => {
            println!("✓ Successfully created ATAs. Transaction signature: {signature}");

            for (mint, ata) in atas_to_create {
                println!("  - Token {mint}: ATA {ata}");
            }
        }
        Err(e) => {
            return Err(KoraError::RpcError(format!(
                "Failed to send ATA creation transaction: {e}"
            )));
        }
    }

    Ok(())
}
