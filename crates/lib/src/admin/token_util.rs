use crate::{
    error::KoraError,
    signer::Signer,
    state::{get_config, get_signer},
    token::token::TokenType,
    transaction::TransactionUtil,
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction, pubkey::Pubkey,
    signature::Signature,
};

use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use std::str::FromStr;

/*
This funciton is tested via the makefile, as it's a CLI command and requires a validator running.
*/

const DEFAULT_CHUNK_SIZE: usize = 10;

pub struct ATAToCreate {
    pub mint: Pubkey,
    pub ata: Pubkey,
    pub token_program: Pubkey,
}

/// Initialize ATAs for all allowed payment tokens for the paymaster
/// This function does not use cache and directly checks on-chain
pub async fn initialize_paymaster_atas(
    rpc_client: &RpcClient,
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    chunk_size: Option<usize>,
) -> Result<(), KoraError> {
    initialize_paymaster_atas_with_chunk_size(
        rpc_client,
        compute_unit_price,
        compute_unit_limit,
        chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
    )
    .await
}

/// Initialize ATAs for all allowed payment tokens for the paymaster with configurable chunk size
/// This function does not use cache and directly checks on-chain
pub async fn initialize_paymaster_atas_with_chunk_size(
    rpc_client: &RpcClient,
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    chunk_size: usize,
) -> Result<(), KoraError> {
    let signer = get_signer()?;
    let config = get_config()?;

    // Determine the payment address
    let payment_address = config.kora.get_payment_address()?;

    println!("Initializing ATAs for payment address: {payment_address}");

    let atas_to_create = find_missing_atas(rpc_client, &payment_address).await?;

    if atas_to_create.is_empty() {
        println!("✓ All required ATAs already exist");
        return Ok(());
    }

    let instructions = atas_to_create
        .iter()
        .map(|ata| {
            create_associated_token_account(
                &signer.solana_pubkey(),
                &payment_address,
                &ata.mint,
                &ata.token_program,
            )
        })
        .collect::<Vec<Instruction>>();

    // Process instructions in chunks
    let total_atas = instructions.len();
    let chunks: Vec<_> = instructions.chunks(chunk_size).collect();
    let num_chunks = chunks.len();

    println!(
        "Creating {total_atas} ATAs in {num_chunks} transaction(s) (chunk size: {chunk_size})..."
    );

    let mut created_atas_idx = 0;

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let chunk_num = chunk_idx + 1;
        println!("Processing chunk {chunk_num}/{num_chunks}");

        // Build instructions for this chunk with compute budget
        let mut chunk_instructions = Vec::new();

        // Add compute budget instructions to each chunk
        if let Some(compute_unit_price) = compute_unit_price {
            chunk_instructions
                .push(ComputeBudgetInstruction::set_compute_unit_price(compute_unit_price));
        }
        if let Some(compute_unit_limit) = compute_unit_limit {
            chunk_instructions
                .push(ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit));
        }

        // Add the ATA creation instructions for this chunk
        chunk_instructions.extend_from_slice(chunk);

        let blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to get blockhash: {e}")))?;

        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &chunk_instructions,
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

        match rpc_client.send_and_confirm_transaction_with_spinner(&tx).await {
            Ok(signature) => {
                println!(
                    "✓ Chunk {chunk_num}/{num_chunks} successful. Transaction signature: {signature}"
                );

                // Print the ATAs created in this chunk
                let chunk_end = std::cmp::min(created_atas_idx + chunk.len(), atas_to_create.len());

                (created_atas_idx..chunk_end).for_each(|i| {
                    let ATAToCreate { mint, ata, token_program } = &atas_to_create[i];
                    println!("  - Token {mint}: ATA {ata} (Token program: {token_program})");
                });
                created_atas_idx = chunk_end;
            }
            Err(e) => {
                return Err(KoraError::RpcError(format!(
                    "Failed to send ATA creation transaction for chunk {chunk_num}/{num_chunks}: {e}"
                )));
            }
        }
    }

    println!("✓ Successfully created all {total_atas} ATAs");

    Ok(())
}

pub async fn find_missing_atas(
    rpc_client: &RpcClient,
    payment_address: &Pubkey,
) -> Result<Vec<ATAToCreate>, KoraError> {
    let config = get_config()?;

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
        return Ok(Vec::new());
    }

    let mut atas_to_create = Vec::new();

    // Check each token mint for existing ATA
    for mint in &token_mints {
        let ata = get_associated_token_address(payment_address, mint);

        match CacheUtil::get_account_from_cache(rpc_client, &ata, false).await {
            Ok(_) => {
                println!("✓ ATA already exists for token {mint}: {ata}");
            }
            Err(_) => {
                // Fetch mint account to determine if it's SPL or Token2022
                let mint_account = CacheUtil::get_account_from_cache(rpc_client, mint, false)
                    .await
                    .map_err(|e| {
                        KoraError::RpcError(format!("Failed to fetch mint account for {mint}: {e}"))
                    })?;

                let token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;

                println!("Creating ATA for token {mint}: {ata}");

                atas_to_create.push(ATAToCreate {
                    mint: *mint,
                    ata,
                    token_program: token_program.program_id(),
                });
            }
        }
    }

    Ok(atas_to_create)
}
