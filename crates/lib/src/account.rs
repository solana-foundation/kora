use super::{cache::TokenAccountCache, get_signer, KoraError, Signer};
use crate::token::TokenInterface;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use std::sync::Arc;

pub async fn get_or_create_token_account(
    rpc_client: &Arc<RpcClient>,
    cache: &TokenAccountCache,
    user_pubkey: &Pubkey,
    mint: &Pubkey,
    token_interface: &impl TokenInterface,
) -> Result<(Pubkey, Option<Transaction>), KoraError> {
    let signer = get_signer()?;

    // Get ATA using spl-associated-token-account
    let ata = get_associated_token_address(user_pubkey, mint);

    // Check cache first
    if let Some(cached_ata) = cache.get_token_account(user_pubkey, mint).await? {
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
            let program_id = token_interface.program_id();
            let create_ata_ix = create_associated_token_account(
                &signer.solana_pubkey(),
                user_pubkey,
                mint,
                &program_id,
            );

            let blockhash = rpc_client.get_latest_blockhash().await.map_err(|e| {
                KoraError::RpcError(format!(
                    "Failed to get blockhash: {e}. Original error: {original_err}"
                ))
            })?;

            let message = Message::new_with_blockhash(
                &[create_ata_ix],
                Some(&signer.solana_pubkey()),
                &blockhash,
            );

            let mut tx = Transaction::new_unsigned(message);
            let signature = signer.sign(&tx).await?;

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
    token_interface: &impl TokenInterface,
) -> Result<(Vec<Pubkey>, Option<Transaction>), KoraError> {
    let signer = get_signer()?;

    let mut atas = Vec::with_capacity(mints.len());
    let mut instructions = Vec::with_capacity(mints.len());
    let mut needs_creation = false;

    for mint in mints {
        let ata = get_associated_token_address(user_pubkey, mint);
        atas.push(ata);

        // Check cache first
        if let Some(_cached_ata) = cache.get_token_account(user_pubkey, mint).await? {
            continue;
        }

        // If not in cache, check on-chain
        if rpc_client.get_account(&ata).await.is_err() {
            needs_creation = true;
            let program_id = token_interface.program_id();
            instructions.push(create_associated_token_account(
                &signer.solana_pubkey(),
                user_pubkey,
                mint,
                &program_id,
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
        .map_err(|e| KoraError::RpcError(format!("Failed to get blockhash: {e}")))?;

    let message =
        Message::new_with_blockhash(&instructions, Some(&signer.solana_pubkey()), &blockhash);

    let mut tx = Transaction::new_unsigned(message);
    let signature = signer.sign(&tx).await?;

    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| KoraError::SigningError("Invalid signature length".to_string()))?;

    let sig = Signature::from(sig_bytes);
    tx.signatures = vec![sig];

    Ok((atas, Some(tx)))
}
