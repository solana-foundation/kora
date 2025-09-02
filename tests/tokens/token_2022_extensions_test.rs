use crate::common::{
    ExtensionHelpers, FeePayerTestHelper, RecipientTestHelper, SenderTestHelper, TestContext,
    TransactionBuilder, USDCMint2022TestHelper, TRANSFER_HOOK_PROGRAM_ID,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;

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

#[tokio::test]
async fn test_transfer_fee_insufficient_payment() {
    // Test that signTransactionIfPaid fails when payment amount doesn't account for transfer fee
    // With 1% transfer fee: sending 1000 tokens results in recipient getting 990 tokens
    // If Kora expects 1000 tokens, the payment should fail validation

    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let mint_keypair = USDCMint2022TestHelper::get_test_usdc_mint_2022_keypair();

    // Create ATAs for sender and fee payer
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

    // Create ATAs if they don't exist
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
        &sender,
        &mint_keypair.pubkey(),
        &sender_ata,
        &sender,
        Some(100_000_000), // 100 USDC (with 6 decimals)
    )
    .await
    .expect("Failed to mint tokens to sender");

    // Build transaction with INSUFFICIENT payment
    // Actual fee: 10,000 lamports = 0.1 USDC equivalent (100,000 micro-USDC equivalent)
    // To make payment insufficient, we send less than what would result in 10,000 lamports after transfer fee
    // If we send 10,000 micro-USDC with 1% transfer fee, recipient gets 9,900 micro-USDC (insufficient)
    let payment_amount = 10_000; // 0.01 USDC in micro-units

    let transaction = TransactionBuilder::v0()
        .with_rpc_client(ctx.rpc_client().clone())
        .with_fee_payer(fee_payer.pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked_with_accounts(
            &mint_keypair.pubkey(),
            &sender_ata,
            &fee_payer_ata,
            &sender.pubkey(),
            payment_amount,
            6,
        )
        .build()
        .await
        .expect("Failed to build transaction");

    // Try to sign the transaction if paid - should fail due to insufficient payment after fees
    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransactionIfPaid", rpc_params![transaction]).await;

    assert!(result.is_err(), "Transaction should have failed due to insufficient payment");

    let error = result.unwrap_err().to_string();

    assert!(
        error.contains("Insufficient payment")
            || error.contains("transfer fee")
            || error.contains("Invalid transaction")
            || error.contains("does not meet the required amount"),
        "Error should mention insufficient payment or transfer fee: {error}",
    );
}

#[tokio::test]
async fn test_transfer_fee_sufficient_payment() {
    // Test that signTransactionIfPaid succeeds when payment amount accounts for transfer fee
    // To receive 10,000 micro-USDC after 1% fee, sender must send ~10,101 micro-USDC

    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let mint_keypair = USDCMint2022TestHelper::get_test_usdc_mint_2022_keypair();

    // Create ATAs for sender and fee payer
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

    // Create ATAs if they don't exist
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
        &sender,
        &mint_keypair.pubkey(),
        &sender_ata,
        &sender,
        Some(100_000_000), // 100 USDC (with 6 decimals)
    )
    .await
    .expect("Failed to mint tokens to sender");

    // Build transaction with SUFFICIENT payment
    // To get 10,000 micro-USDC after 1% fee, we need to send:
    // amount / (1 - 0.01) = 10,000 / 0.99 ≈ 10,101
    let payment_amount = 10_101; // This should result in ~10,000 after 1% fee

    let transaction = TransactionBuilder::v0()
        .with_rpc_client(ctx.rpc_client().clone())
        .with_fee_payer(fee_payer.pubkey())
        .with_signer(&sender)
        .with_spl_token_2022_transfer_checked_with_accounts(
            &mint_keypair.pubkey(),
            &sender_ata,
            &fee_payer_ata,
            &sender.pubkey(),
            payment_amount,
            6,
        )
        .build()
        .await
        .expect("Failed to build transaction");

    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransactionIfPaid", rpc_params![transaction]).await;

    assert!(
        result.is_ok(),
        "Transaction should have succeeded with sufficient payment: {:?}",
        result.unwrap_err()
    );

    let response = result.unwrap();

    assert!(
        response.get("signed_transaction").is_some(),
        "Response should contain signed_transaction"
    );
}

// **************************************************************************************
// Token 2022 Transfer Hook Tests
// **************************************************************************************

/// Test Token 2022 transfer with transfer hook that allows transfers
#[tokio::test]
async fn test_transfer_hook_allows_transfer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let hook_program_id =
        Pubkey::from_str(TRANSFER_HOOK_PROGRAM_ID).expect("Invalid transfer hook program ID");

    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let transfer_hook_mint_keypair = USDCMint2022TestHelper::get_test_transfer_hook_mint_keypair();

    ExtensionHelpers::create_mint_with_transfer_hook(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair,
        &hook_program_id,
    )
    .await
    .expect("Failed to create mint with transfer hook");

    // Create ATAs for sender and recipient for the transfer hook mint
    let sender_ata = get_associated_token_address_with_program_id(
        &sender.pubkey(),
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    let recipient_ata = get_associated_token_address_with_program_id(
        &recipient,
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    // Create ATAs
    let create_sender_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &sender.pubkey(),
            &transfer_hook_mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let create_recipient_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &recipient,
            &transfer_hook_mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let create_atas_tx = Transaction::new_signed_with_payer(
        &[create_sender_ata, create_recipient_ata],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    rpc_client.send_and_confirm_transaction(&create_atas_tx).await.expect("Failed to create ATAs");

    ExtensionHelpers::mint_tokens_to_account(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair.pubkey(),
        &sender_ata,
        &fee_payer,
        Some(1_000_000), // 1M tokens
    )
    .await
    .expect("Failed to mint tokens to sender");

    let mut transfer_instruction = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022::id(),
        &sender_ata,
        &transfer_hook_mint_keypair.pubkey(),
        &recipient_ata,
        &sender.pubkey(),
        &[],
        10,
        6,
    )
    .expect("Failed to create transfer_checked instruction");

    // Get the Extra Account Meta List address for the transfer hook
    let extra_account_metas_address = spl_transfer_hook_interface::get_extra_account_metas_address(
        &transfer_hook_mint_keypair.pubkey(),
        &hook_program_id,
    );

    // Add the extra account metas list as a read-only account
    transfer_instruction
        .accounts
        .push(AccountMeta::new_readonly(extra_account_metas_address, false));

    // Add the transfer hook program itself as a read-only account
    transfer_instruction.accounts.push(AccountMeta::new_readonly(hook_program_id, false));

    // Create transaction with manual instruction
    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let test_transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer, &sender],
        recent_blockhash,
    );

    // Encode as base64 for Kora RPC
    let serialized = bincode::serialize(&test_transaction).unwrap();
    let test_tx = STANDARD.encode(serialized);

    // Submit to Kora - should succeed because hook allows transfers
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction with transfer hook");

    assert!(response["signature"].as_str().is_some(), "Expected signature in response");
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Verify transaction would simulate successfully
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transfer hook transaction");

    // Transaction should succeed with transfer hook allowing transfer
    assert!(
        simulated_tx.value.err.is_none(),
        "Transfer hook transaction simulation should succeed: {:?}",
        simulated_tx.value.err
    );
}

/// Test Token 2022 transfer with transfer hook that blocks transfers
#[tokio::test]
async fn test_transfer_hook_blocks_transfer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let hook_program_id =
        Pubkey::from_str(TRANSFER_HOOK_PROGRAM_ID).expect("Invalid transfer hook program ID");

    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let transfer_hook_mint_keypair = USDCMint2022TestHelper::get_test_transfer_hook_mint_keypair();

    ExtensionHelpers::create_mint_with_transfer_hook(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair,
        &hook_program_id,
    )
    .await
    .expect("Failed to create mint with transfer hook");

    // Create ATAs for sender and recipient for the transfer hook mint
    let sender_ata = get_associated_token_address_with_program_id(
        &sender.pubkey(),
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    let recipient_ata = get_associated_token_address_with_program_id(
        &recipient,
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022::id(),
    );

    // Create ATAs
    let create_sender_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &sender.pubkey(),
            &transfer_hook_mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let create_recipient_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &fee_payer.pubkey(),
            &recipient,
            &transfer_hook_mint_keypair.pubkey(),
            &spl_token_2022::id(),
        );

    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let create_atas_tx = Transaction::new_signed_with_payer(
        &[create_sender_ata, create_recipient_ata],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    rpc_client.send_and_confirm_transaction(&create_atas_tx).await.expect("Failed to create ATAs");

    // Mint tokens to sender (more than the hook limit of 1M)
    ExtensionHelpers::mint_tokens_to_account(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair.pubkey(),
        &sender_ata,
        &fee_payer,
        Some(5_000_000), // 5M tokens - much more than needed
    )
    .await
    .expect("Failed to mint tokens to sender");

    // Create transfer instruction manually with transfer hook accounts - large amount that will be blocked
    let mut transfer_instruction = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022::id(),
        &sender_ata,
        &transfer_hook_mint_keypair.pubkey(),
        &recipient_ata,
        &sender.pubkey(),
        &[],
        2_000_000, // Large amount that exceeds our 1M limit
        6,
    )
    .expect("Failed to create transfer_checked instruction");

    // Get the Extra Account Meta List address for the transfer hook
    let extra_account_metas_address = spl_transfer_hook_interface::get_extra_account_metas_address(
        &transfer_hook_mint_keypair.pubkey(),
        &hook_program_id,
    );

    // Add the extra account metas list as a read-only account
    transfer_instruction
        .accounts
        .push(AccountMeta::new_readonly(extra_account_metas_address, false));

    // Add the transfer hook program itself as a read-only account
    transfer_instruction.accounts.push(AccountMeta::new_readonly(hook_program_id, false));

    // Create transaction with manual instruction
    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let test_transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer, &sender],
        recent_blockhash,
    );

    let serialized = bincode::serialize(&test_transaction).unwrap();
    let test_tx = STANDARD.encode(serialized);

    // Submit to Kora - transaction should be rejected by transfer hook
    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransaction", rpc_params![test_tx]).await;

    // The call should fail because the transfer hook blocks large transfers
    assert!(
        result.is_err(),
        "Expected signTransaction to fail due to transfer hook blocking large transfer"
    );

    let error_message = format!("{:?}", result.unwrap_err());

    // Verify the error contains the expected transfer hook error code
    assert!(
        error_message.contains("custom program error: 0x1") || error_message.contains("Custom(1)"),
        "Expected error to contain transfer hook error code 0x1, got: {error_message}",
    );
}
