use crate::{
    config::Config,
    error::KoraError,
    state::{get_request_signer_with_signer_key, get_signer_pool},
    token::token::TokenType,
    transaction::TransactionUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_keychain::SolanaSigner;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use spl_associated_token_account_interface::{
    address::get_associated_token_address, instruction::create_associated_token_account,
};
use std::{fmt::Display, str::FromStr, sync::Arc};

#[cfg(not(test))]
use {crate::cache::CacheUtil, crate::state::get_config};

#[cfg(test)]
use {
    crate::config::SplTokenConfig, crate::tests::cache_mock::MockCacheUtil as CacheUtil,
    crate::tests::config_mock::mock_state::get_config,
};

/*
This funciton is tested via the makefile, as it's a CLI command and requires a validator running.
*/

const DEFAULT_CHUNK_SIZE: usize = 10;

pub struct ATAToCreate {
    pub mint: Pubkey,
    pub ata: Pubkey,
    pub token_program: Pubkey,
}

impl Display for ATAToCreate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token {}: ATA {} (Token program: {})", self.mint, self.ata, self.token_program)
    }
}

/// Initialize ATAs for all allowed payment tokens for the paymaster
/// This function initializes ATAs for ALL signers in the pool
///
/// Order of priority is:
/// 1. Payment address provided in config
/// 2. All signers in pool
pub async fn initialize_atas(
    rpc_client: &RpcClient,
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    chunk_size: Option<usize>,
    fee_payer_key: Option<String>,
) -> Result<(), KoraError> {
    let config = get_config()?;

    let fee_payer = get_request_signer_with_signer_key(fee_payer_key.as_deref())?;

    let addresses_to_initialize_atas = if let Some(payment_address) = &config.kora.payment_address {
        vec![Pubkey::from_str(payment_address)
            .map_err(|e| KoraError::InternalServerError(format!("Invalid payment address: {e}")))?]
    } else {
        get_signer_pool()?
            .get_signers_info()
            .iter()
            .filter_map(|info| info.public_key.parse().ok())
            .collect::<Vec<Pubkey>>()
    };

    initialize_atas_with_chunk_size(
        rpc_client,
        &fee_payer,
        &addresses_to_initialize_atas,
        compute_unit_price,
        compute_unit_limit,
        chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
    )
    .await
}

/// Initialize ATAs for all allowed payment tokens for the provided addresses with configurable chunk size
/// This function does not use cache and directly checks on-chain
pub async fn initialize_atas_with_chunk_size(
    rpc_client: &RpcClient,
    fee_payer: &Arc<solana_keychain::Signer>,
    addresses_to_initialize_atas: &Vec<Pubkey>,
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    chunk_size: usize,
) -> Result<(), KoraError> {
    let config = get_config()?;

    for address in addresses_to_initialize_atas {
        println!("Initializing ATAs for address: {address}");

        #[allow(clippy::needless_borrow)]
        let atas_to_create = find_missing_atas(&config, rpc_client, address).await?;

        if atas_to_create.is_empty() {
            println!("✓ All required ATAs already exist for address: {address}");
            continue;
        }

        create_atas_for_signer(
            rpc_client,
            fee_payer,
            address,
            &atas_to_create,
            compute_unit_price,
            compute_unit_limit,
            chunk_size,
        )
        .await?;
    }

    println!("✓ Successfully created all ATAs");

    Ok(())
}

/// Helper function to create ATAs for a single signer
async fn create_atas_for_signer(
    rpc_client: &RpcClient,
    fee_payer: &Arc<solana_keychain::Signer>,
    address: &Pubkey,
    atas_to_create: &[ATAToCreate],
    compute_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
    chunk_size: usize,
) -> Result<usize, KoraError> {
    let instructions = atas_to_create
        .iter()
        .map(|ata| {
            create_associated_token_account(
                &fee_payer.pubkey(),
                address,
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

        let fee_payer_pubkey = fee_payer.pubkey();
        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &chunk_instructions,
            Some(&fee_payer_pubkey),
            &blockhash,
        ));

        let mut tx = TransactionUtil::new_unsigned_versioned_transaction(message);
        let message_bytes = tx.message.serialize();
        let signature = fee_payer
            .sign_message(&message_bytes)
            .await
            .map_err(|e| KoraError::SigningError(e.to_string()))?;

        tx.signatures = vec![signature];

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
                println!("✗ Chunk {chunk_num}/{num_chunks} failed: {e}");

                if created_atas_idx > 0 {
                    println!("\nSuccessfully created ATAs ({created_atas_idx}/{total_atas}):");
                    println!(
                        "{}",
                        atas_to_create[0..created_atas_idx]
                            .iter()
                            .map(|ata| format!("  ✓ {ata}"))
                            .collect::<Vec<String>>()
                            .join("\n")
                    );
                    println!("\nRemaining ATAs to create: {}", total_atas - created_atas_idx);
                } else {
                    println!("No ATAs were successfully created.");
                }

                println!("This may be a temporary network issue. Please re-run the command to retry ATA creation.");
                return Err(KoraError::RpcError(format!(
                    "Failed to send ATA creation transaction for chunk {chunk_num}/{num_chunks}: {e}"
                )));
            }
        }
    }

    // Show summary of all successfully created ATAs
    println!("\n🎉 All ATA creation completed successfully!");
    println!("Successfully created ATAs ({total_atas}/{total_atas}):");
    println!(
        "{}",
        atas_to_create.iter().map(|ata| format!("  ✓ {ata}")).collect::<Vec<String>>().join("\n")
    );

    Ok(total_atas)
}

pub async fn find_missing_atas(
    config: &Config,
    rpc_client: &RpcClient,
    payment_address: &Pubkey,
) -> Result<Vec<ATAToCreate>, KoraError> {
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

        match CacheUtil::get_account(config, rpc_client, &ata, false).await {
            Ok(_) => {
                println!("✓ ATA already exists for token {mint}: {ata}");
            }
            Err(_) => {
                // Fetch mint account to determine if it's SPL or Token2022
                let mint_account =
                    CacheUtil::get_account(config, rpc_client, mint, false).await.map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        common::{
            create_mock_rpc_client_account_not_found, create_mock_spl_mint_account,
            create_mock_token_account, setup_or_get_test_signer, RpcMockBuilder,
        },
        config_mock::{ConfigMockBuilder, ValidationConfigBuilder},
    };
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    #[tokio::test]
    async fn test_find_missing_atas_no_spl_tokens() {
        let _m = ConfigMockBuilder::new()
            .with_validation(
                ValidationConfigBuilder::new()
                    .with_allowed_spl_paid_tokens(SplTokenConfig::Allowlist(vec![]))
                    .build(),
            )
            .build_and_setup();

        let rpc_client = create_mock_rpc_client_account_not_found();
        let payment_address = Pubkey::new_unique();

        let config = get_config().unwrap();
        let result = find_missing_atas(&config, &rpc_client, &payment_address).await.unwrap();

        assert!(result.is_empty(), "Should return empty vec when no SPL tokens configured");
    }

    #[tokio::test]
    async fn test_find_missing_atas_with_spl_tokens() {
        let allowed_spl_tokens = [Pubkey::new_unique(), Pubkey::new_unique()];

        let _m = ConfigMockBuilder::new()
            .with_validation(
                ValidationConfigBuilder::new()
                    .with_allowed_spl_paid_tokens(SplTokenConfig::Allowlist(
                        allowed_spl_tokens.iter().map(|p| p.to_string()).collect(),
                    ))
                    .build(),
            )
            .build_and_setup();

        let cache_ctx = CacheUtil::get_account_context();
        cache_ctx.checkpoint(); // Clear any previous expectations

        let payment_address = Pubkey::new_unique();
        let rpc_client = create_mock_rpc_client_account_not_found();

        // First call: Found in cache (Ok)
        // Second call: ATA account not found (Err)
        // Third call: mint account found (Ok)
        let responses = Arc::new(Mutex::new(VecDeque::from([
            Ok(create_mock_token_account(&Pubkey::new_unique(), &Pubkey::new_unique())),
            Err(KoraError::RpcError("ATA not found".to_string())),
            Ok(create_mock_spl_mint_account(6)),
        ])));

        let responses_clone = responses.clone();
        cache_ctx
            .expect()
            .times(3)
            .returning(move |_, _, _, _| responses_clone.lock().unwrap().pop_front().unwrap());

        let config = get_config().unwrap();
        let result = find_missing_atas(&config, &rpc_client, &payment_address).await;

        assert!(result.is_ok(), "Should handle SPL tokens with proper mocking");
        let atas = result.unwrap();
        assert_eq!(atas.len(), 1, "Should return 1 missing ATAs");
    }

    #[tokio::test]
    async fn test_create_atas_for_signer_calls_rpc_correctly() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let _ = setup_or_get_test_signer();

        let address = Pubkey::new_unique();
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();

        let atas_to_create = vec![
            ATAToCreate {
                mint: mint1,
                ata: spl_associated_token_account_interface::address::get_associated_token_address(
                    &address, &mint1,
                ),
                token_program: spl_token_interface::id(),
            },
            ATAToCreate {
                mint: mint2,
                ata: spl_associated_token_account_interface::address::get_associated_token_address(
                    &address, &mint2,
                ),
                token_program: spl_token_interface::id(),
            },
        ];

        let rpc_client = RpcMockBuilder::new().with_blockhash().with_send_transaction().build();

        let result = create_atas_for_signer(
            &rpc_client,
            &get_request_signer_with_signer_key(None).unwrap(),
            &address,
            &atas_to_create,
            Some(1000),
            Some(100_000),
            2,
        )
        .await;

        // Should fail with signature validation error since mock signature doesn't match real transaction
        match result {
            Ok(_) => {
                panic!("Expected signature validation error, but got success");
            }
            Err(e) => {
                let error_msg = format!("{e:?}");
                // Check if it's a signature validation error (the mocked signature doesn't match the real transaction signature)
                assert!(
                    error_msg.contains("signature")
                        || error_msg.contains("Signature")
                        || error_msg.contains("invalid")
                        || error_msg.contains("mismatch"),
                    "Expected signature validation error, got: {error_msg}"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_initialize_atas_when_all_tokens_are_allowed() {
        let _m = ConfigMockBuilder::new()
            .with_allowed_spl_paid_tokens(SplTokenConfig::All)
            .build_and_setup();

        let _ = setup_or_get_test_signer();

        let rpc_client = RpcMockBuilder::new().build();

        let result = initialize_atas(&rpc_client, None, None, None, None).await;

        assert!(result.is_ok(), "Expected atas init to succeed");
    }
}
