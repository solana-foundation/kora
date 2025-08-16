use crate::common::*;
use kora_lib::{
    token::{Token2022Account, Token2022Program, TokenInterface},
    transaction::TransactionUtil,
};
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
};
use std::str::FromStr;

#[tokio::test]
async fn test_pyusd_token_e2e_with_kora() {
    // Get RPC client
    let rpc_client = RPCTestHelper::get_rpc_client().await;

    // Create a token program interface for Token2022
    let token_program = Token2022Program::new();

    // PYUSD mint on devnet
    let pyusd_mint = Pubkey::from_str(PYUSD_MINT).unwrap();

    // Create a test wallet
    let wallet = Keypair::new();

    // Get associated token address for this wallet and PYUSD using the TokenInterface
    let token_account_address =
        token_program.get_associated_token_address(&wallet.pubkey(), &pyusd_mint);

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
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[destination_ata_ix, transfer_ix],
        Some(&wallet.pubkey()),
        &recent_blockhash,
    ));

    // For a real token account, we'd need to query the account data
    if let Ok(account) = rpc_client.get_account(&token_account_address).await {
        if !account.data.is_empty() {
            // Unpack the token account data using the Token2022Program
            let original_token_program = Token2022Program::new();
            let token_state = original_token_program.unpack_token_account(&account.data).unwrap();

            // Verify it's a Token2022Account
            if let Some(token2022_account) = token_state.as_any().downcast_ref::<Token2022Account>()
            {
                // Validate token extensions using the interface method
                let validation_result = original_token_program
                    .get_and_validate_amount_for_payment(
                        &rpc_client,
                        Some(token2022_account),
                        None,
                        transfer_amount,
                    )
                    .await;
                assert!(
                    validation_result.is_ok(),
                    "Token2022Account validation failed: {validation_result:?}"
                );
            }
        }
    }
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
    let message = VersionedMessage::Legacy(Message::new(
        &[create_ata_ix, transfer_ix],
        Some(&wallet.pubkey()),
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    // Verify transaction structure
    assert_eq!(transaction.message.instructions().len(), 2);
    assert_eq!(transaction.message.header().num_required_signatures, 1);
}
