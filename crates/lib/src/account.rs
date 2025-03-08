use crate::token::TokenType;

use super::{cache::TokenAccountCache, get_signer, KoraError, Signer};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use std::sync::Arc;

pub async fn get_or_create_token_account(
    rpc_client: &Arc<RpcClient>,
    cache: &TokenAccountCache,
    user_pubkey: &Pubkey,
    mint: &Pubkey,
) -> Result<(Pubkey, Option<Transaction>), KoraError> {
    let signer = get_signer()?;

    // Get token instance
    let token = TokenType::try_from_mint(rpc_client, mint).await?;

    // Get ATA using spl-associated-token-account
    let ata = token.get_associated_token_address(user_pubkey, mint);

    // Check cache first
    if let Some(cached_ata) = cache.get_token_account(user_pubkey, &token.id()).await? {
        return Ok((cached_ata, None));
    }

    // If not in cache, check on-chain
    match rpc_client.get_account(&ata).await {
        Ok(_) => {
            // Account exists, cache it and return
            cache.set_token_account(user_pubkey, mint, &ata).await?;
            Ok((ata, None))
        }
        Err(original_err) => {
            // Account doesn't exist, create it
            let create_ata_ix = token.create_associated_token_account(
                &signer.solana_pubkey(),
                user_pubkey,
                &token.token_program().id(),
                mint,
            );

            let blockhash = rpc_client.get_latest_blockhash().await.map_err(|e| {
                KoraError::RpcError(format!(
                    "Failed to get blockhash: {}. Original error: {}",
                    e, original_err
                ))
            })?;

            let message = Message::new_with_blockhash(
                &[create_ata_ix],
                Some(&signer.solana_pubkey()),
                &blockhash,
            );

            let mut tx = Transaction::new_unsigned(message);
            let signature = signer.sign(&tx.message_data()).await?;

            let sig_bytes: [u8; 64] = signature
                .bytes
                .try_into()
                .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

            let sig = Signature::from(sig_bytes);
            tx.signatures = vec![sig];

            Ok((ata, Some(tx)))
        }
    }
}

pub async fn get_or_create_multiple_token_accounts(
    rpc_client: &Arc<RpcClient>,
    cache: &TokenAccountCache,
    user_pubkey: &Pubkey,
    mints: &[Pubkey],
) -> Result<(Vec<Pubkey>, Option<Transaction>), KoraError> {
    let signer = get_signer()?;

    let mut atas = Vec::with_capacity(mints.len());
    let mut instructions = Vec::with_capacity(mints.len());
    let mut needs_creation = false;

    for mint in mints {
        let token = TokenType::try_from_mint(rpc_client, mint).await?;
        let ata = token.get_associated_token_address(user_pubkey, mint);
        atas.push(ata);

        // Check cache first
        if let Some(_cached_ata) = cache.get_token_account(user_pubkey, mint).await? {
            continue;
        }

        // If not in cache, check on-chain
        if rpc_client.get_account(&ata).await.is_err() {
            needs_creation = true;
            instructions.push(token.create_associated_token_account(
                &signer.solana_pubkey(),
                user_pubkey,
                &token.token_program().id(),
                mint,
            ));
        } else {
            // Account exists, cache it
            cache.set_token_account(user_pubkey, mint, &ata).await?;
        }
    }

    if !needs_creation {
        return Ok((atas, None));
    }

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to get blockhash: {}", e)))?;

    let message =
        Message::new_with_blockhash(&instructions, Some(&signer.solana_pubkey()), &blockhash);

    let mut tx = Transaction::new_unsigned(message);
    let signature = signer.sign(&tx.message_data()).await?;

    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

    let sig = Signature::from(sig_bytes);
    tx.signatures = vec![sig];

    Ok((atas, Some(tx)))
}
