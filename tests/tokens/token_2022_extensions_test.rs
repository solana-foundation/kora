use crate::common::{
    ExtensionHelpers, FeePayerTestHelper, TestContext, TransactionBuilder, USDCMint2022TestHelper,
};
use jsonrpsee::rpc_params;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use tests::common::SenderTestHelper;

#[tokio::test]
async fn test_blocked_memo_transfer_extension() {
    // This test creates manual token accounts with MemoTransfer extension
    // Should be blocked by kora-test.toml when using token accounts with MemoTransfer extension

    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let mint_keypair = USDCMint2022TestHelper::get_test_usdc_mint_2022_keypair();

    let sender_token_account = Keypair::new();

    // Create manual token accounts with MemoTransfer extension
    ExtensionHelpers::create_token_account_with_memo_transfer(
        ctx.rpc_client(),
        &sender,
        &sender_token_account,
        &mint_keypair.pubkey(),
        &sender,
    )
    .await
    .expect("Failed to create sender token account");

    let fee_payer_token_account = get_associated_token_address_with_program_id(
        &fee_payer.pubkey(),
        &mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    // Create recipient ATA for custom mint (normal ATA without MemoTransfer extension)
    let create_fee_payer_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &fee_payer.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let create_fee_payer_payment_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &fee_payer.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let recent_blockhash = ctx.rpc_client().get_latest_blockhash().await.unwrap();
    let create_atas_transaction = Transaction::new_signed_with_payer(
        &[create_fee_payer_ata_instruction, create_fee_payer_payment_ata_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    ctx.rpc_client()
        .send_and_confirm_transaction(&create_atas_transaction)
        .await
        .expect("Failed to create ATAs");

    // Mint tokens to sender account for the main transfer
    ExtensionHelpers::mint_tokens_to_account(
        ctx.rpc_client(),
        &sender,
        &mint_keypair.pubkey(),
        &sender_token_account.pubkey(),
        &sender,
        Some(1_000_000),
    )
    .await
    .expect("Failed to mint tokens");

    // Build transaction with manual token accounts that have MemoTransfer extension
    let transaction = TransactionBuilder::v0()
        .with_rpc_client(ctx.rpc_client().clone())
        .with_fee_payer(fee_payer.pubkey())
        .with_signer(&sender)
        // Payment instructions
        .with_spl_token_2022_transfer_checked_with_accounts(
            &mint_keypair.pubkey(),
            &sender_token_account.pubkey(),
            &fee_payer_token_account,
            &sender.pubkey(),
            1_000_000,
            6,
        )
        .build()
        .await
        .expect("Failed to build transaction");

    // Try to sign the transaction if paid - should fail due to blocked MemoTransfer on token accounts
    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransactionIfPaid", rpc_params![transaction]).await;

    // This should fail when disallowed_token_extensions includes "MemoTransfer"
    assert!(result.is_err(), "Transaction should have failed");

    let error = result.unwrap_err().to_string();

    assert!(
        error.contains("Blocked account extension found on source account"),
        "Error should mention blocked extension: {error}",
    );
}

#[tokio::test]
async fn test_blocked_interest_bearing_config_extension() {
    // This test creates a mint with InterestBearingConfig extension on-demand
    // Should be blocked by kora-test.toml when using mint with InterestBearingConfig extension

    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();

    // Create mint with InterestBearingConfig extension
    let mint_keypair = USDCMint2022TestHelper::get_test_interest_bearing_mint_keypair();

    // Create mint with InterestBearingConfig extension
    ExtensionHelpers::create_mint_with_interest_bearing(
        ctx.rpc_client(),
        &fee_payer,
        &mint_keypair,
    )
    .await
    .expect("Failed to create mint with interest bearing");

    // Create ATAs for sender and fee payer since it's a new mint
    let sender_ata = get_associated_token_address_with_program_id(
        &sender.pubkey(),
        &mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    let fee_payer_ata = get_associated_token_address_with_program_id(
        &fee_payer.pubkey(),
        &mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    let create_sender_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &sender.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let create_fee_payer_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &fee_payer.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let recent_blockhash = ctx.rpc_client().get_latest_blockhash().await.unwrap();
    let create_atas_transaction = Transaction::new_signed_with_payer(
        &[create_sender_ata_instruction, create_fee_payer_ata_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    ctx.rpc_client()
        .send_and_confirm_transaction(&create_atas_transaction)
        .await
        .expect("Failed to create ATAs");

    // Mint tokens to sender
    ExtensionHelpers::mint_tokens_to_account(
        ctx.rpc_client(),
        &fee_payer,
        &mint_keypair.pubkey(),
        &sender_ata,
        &fee_payer,
        Some(1_000_000),
    )
    .await
    .expect("Failed to mint tokens to sender");

    // Use regular ATAs for the transfer (no blocked token account extensions)
    // This way we test ONLY the mint extension blocking (InterestBearingConfig)
    let transaction = TransactionBuilder::v0()
        .with_rpc_client(ctx.rpc_client().clone())
        .with_fee_payer(fee_payer.pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked_with_accounts(
            &mint_keypair.pubkey(),
            &sender_ata,
            &fee_payer_ata,
            &sender.pubkey(),
            1_000_000,
            6,
        )
        .build()
        .await
        .expect("Failed to build transaction");

    // Try to sign the transaction if paid - should fail due to blocked InterestBearingConfig on mint
    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransactionIfPaid", rpc_params![transaction]).await;

    // This should fail when disallowed_mint_extensions includes "InterestBearingConfig"
    assert!(result.is_err(), "Transaction should have failed");

    let error = result.unwrap_err().to_string();

    assert!(
        error.contains("Blocked mint extension found on mint"),
        "Error should mention blocked extension: {error}",
    );
}
