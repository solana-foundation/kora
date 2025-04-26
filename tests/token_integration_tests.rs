use kora_lib::{
    token::{Token2022Account, Token2022Program, TokenInterface},
    transaction::validator::validate_token2022_account,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_2022;
use std::str::FromStr;

// PYUSD token mint on devnet
const PYUSD_MINT: &str = "CXk2AMBfi3TwaEL2468s6zP8xq9NxTXjp9gjMgzeUynM";

#[tokio::test]
async fn test_pyusd_token_e2e_with_kora() {
    // Get a connection to devnet
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Create a token program interface for Token2022
    let token_program = Token2022Program::new();

    // PYUSD mint on devnet
    let pyusd_mint = Pubkey::from_str(PYUSD_MINT).unwrap();

    // Create a test wallet
    let wallet = Keypair::new();
    println!("Test wallet address: {}", wallet.pubkey());

    // Get associated token address for this wallet and PYUSD using the TokenInterface
    let token_account_address =
        token_program.get_associated_token_address(&wallet.pubkey(), &pyusd_mint);
    println!("Token account address: {}", token_account_address);

    // Create a simulated transfer instruction
    let transfer_amount = 1_000_000; // 1 PYUSD with 6 decimals
    let decimals = 6; // PYUSD has 6 decimals

    let destination_wallet = Keypair::new();

    // Create the destination token account first
    let destination_ata_ix = token_program.create_associated_token_account_instruction(
        &wallet.pubkey(),
        &destination_wallet.pubkey(),
        &pyusd_mint,
    );

    // Get destination token address using the TokenInterface
    let destination =
        token_program.get_associated_token_address(&destination_wallet.pubkey(), &pyusd_mint);

    // Create a transfer instruction using the TokenInterface directly
    let transfer_ix = token_program
        .create_transfer_checked_instruction(
            &token_account_address,
            &pyusd_mint,
            &destination,
            &wallet.pubkey(),
            transfer_amount,
            decimals,
        )
        .unwrap();

    // Get a new recent blockhash
    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();

    // Create a transaction for the transfer that includes creating the destination account
    let transfer_tx = Transaction::new_signed_with_payer(
        &[destination_ata_ix, transfer_ix],
        Some(&wallet.pubkey()),
        &[&wallet],
        recent_blockhash,
    );

    // Validate the transaction using Kora's validator
    let validation_config = kora_lib::config::ValidationConfig {
        max_allowed_lamports: 1000000000,
        max_signatures: 10,
        allowed_programs: vec![
            spl_token_2022::id().to_string(),
            spl_associated_token_account::id().to_string(),
        ],
        allowed_tokens: vec![pyusd_mint.to_string()],
        allowed_spl_paid_tokens: vec![],
        disallowed_accounts: vec![],
        price_source: kora_lib::oracle::PriceSource::Jupiter,
    };

    let validation_result = kora_lib::transaction::validator::TransactionValidator::new(
        wallet.pubkey(),
        &validation_config,
    )
    .unwrap()
    .validate_transaction(&transfer_tx);

    // Assert the transaction is valid according to Kora rules
    assert!(
        validation_result.is_ok(),
        "Expected transfer transaction to be valid: {:?}",
        validation_result
    );

    // For a real token account, we'd need to query the account data
    if let Ok(account) = rpc_client.get_account(&token_account_address).await {
        if !account.data.is_empty() {
            // Unpack the token account data using the Token2022Program
            let original_token_program = Token2022Program::new();
            let token_state = original_token_program.unpack_token_account(&account.data).unwrap();

            // Verify it's a Token2022Account
            if let Some(token2022_account) = token_state.as_any().downcast_ref::<Token2022Account>()
            {
                // Validate token extensions
                let validation_result =
                    validate_token2022_account(token2022_account, transfer_amount);
                assert!(
                    validation_result.is_ok(),
                    "Token2022Account validation failed: {:?}",
                    validation_result
                );
            }
        }
    }

    println!("PYUSD token e2e test transaction validation successful!");
}

// The basic test for token operations
#[test]
fn test_token2022_operations() {
    // Create a token program for testing
    let token_program = Token2022Program::new();

    // Test wallet creation
    let wallet = Keypair::new();
    let mint = Pubkey::new_unique(); // Use a random mint for testing

    // Create instructions for token operations using TokenInterface
    let ata = token_program.get_associated_token_address(&wallet.pubkey(), &mint);
    let create_ata_ix = token_program.create_associated_token_account_instruction(
        &wallet.pubkey(),
        &wallet.pubkey(),
        &mint,
    );

    // For testing, we need to handle the instruction creation directly since
    // our mint isn't a real mint on the blockchain
    let destination = Pubkey::new_unique();

    // Use TokenInterface directly to create the instruction
    let transfer_ix = token_program
        .create_transfer_checked_instruction(&ata, &mint, &destination, &wallet.pubkey(), 100, 9)
        .unwrap();

    // Create a transaction with both instructions
    let transaction =
        Transaction::new_with_payer(&[create_ata_ix, transfer_ix], Some(&wallet.pubkey()));

    // Verify transaction structure
    assert_eq!(transaction.message.instructions.len(), 2);
    assert_eq!(transaction.message.header.num_required_signatures, 1);
}
