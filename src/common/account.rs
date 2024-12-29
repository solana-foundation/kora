use super::{get_signer, Signer};
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::instruction::initialize_account;
use std::sync::Arc;

pub async fn create_token_account(
    rpc_client: &Arc<RpcClient>,
    user_pubkey: &Pubkey,
    mint: &Pubkey,
) -> Result<(Pubkey, Option<Transaction>)> {
    let signer = get_signer().map_err(|e| anyhow::anyhow!("Failed to get signer: {}", e))?;

    // Get ATA using spl-associated-token-account
    let ata = get_associated_token_address(user_pubkey, mint);

    match rpc_client.get_account(&ata).await {
        Ok(_) => Ok((ata, None)),
        Err(_) => {
            let create_ata_ix =
                initialize_account(&signer.solana_pubkey(), user_pubkey, mint, user_pubkey)
                    .map_err(|e| anyhow::anyhow!("Failed to initialize account: {}", e))?;

            let blockhash = rpc_client
                .get_latest_blockhash()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {}", e))?;

            let message = Message::new_with_blockhash(
                &[create_ata_ix],
                Some(&signer.solana_pubkey()),
                &blockhash,
            );

            let mut tx = Transaction::new_unsigned(message);
            let signature = signer
                .partial_sign(&tx.message_data())
                .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;

            let sig_bytes: [u8; 64] = signature
                .bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

            let sig = Signature::from(sig_bytes);
            tx.signatures = vec![sig];

            Ok((ata, Some(tx)))
        }
    }
}

pub async fn create_multiple_token_accounts(
    rpc_client: &Arc<RpcClient>,
    user_pubkey: &Pubkey,
    mints: &[Pubkey],
) -> Result<(Vec<Pubkey>, Option<Transaction>)> {
    let signer = get_signer().map_err(|e| anyhow::anyhow!("Failed to get signer: {}", e))?;

    let mut atas = Vec::with_capacity(mints.len());
    let mut instructions = Vec::with_capacity(mints.len());
    let mut needs_creation = false;

    for mint in mints {
        let ata = get_associated_token_address(user_pubkey, mint);
        atas.push(ata);

        if rpc_client.get_account(&ata).await.is_err() {
            needs_creation = true;
            instructions.push(create_associated_token_account(
                &signer.solana_pubkey(),
                user_pubkey,
                mint,
                &spl_token::id(),
            ));
        }
    }

    if !needs_creation {
        return Ok((atas, None));
    }

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {}", e))?;

    let message =
        Message::new_with_blockhash(&instructions, Some(&signer.solana_pubkey()), &blockhash);

    let mut tx = Transaction::new_unsigned(message);
    let signature = signer
        .partial_sign(&tx.message_data())
        .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;

    let sig_bytes: [u8; 64] = signature
        .bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

    let sig = Signature::from(sig_bytes);
    tx.signatures = vec![sig];

    Ok((atas, Some(tx)))
}
