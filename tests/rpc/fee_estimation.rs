use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_estimate_transaction_fee_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
}

/// Test estimateTransactionFee without fee token parameter
#[tokio::test]
async fn test_estimate_transaction_fee_without_fee_token_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");

    assert!(
        response["fee_in_token"].is_null(),
        "Expected fee_in_token to be null when not requested"
    );
}

/// Test estimateTransactionFee with fee token parameter
#[tokio::test]
async fn test_estimate_transaction_fee_with_fee_token_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string();

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx, usdc_mint])
        .await
        .expect("Failed to estimate transaction fee with token");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");
    response.assert_has_field("fee_in_token");

    let fee_in_lamports = response["fee_in_lamports"].as_u64().unwrap();
    let fee_in_token = response["fee_in_token"].as_f64().unwrap();

    // Verify the conversion makes sense
    // Mocked price DEFAULT_MOCKED_PRICE is 0.001, so 0.001 SOL per usdc
    // Fee in lamport is 10050
    // 10000 lamports -> 0.00001 SOL -> 0.00001 / 0.001 (sol per usdc) = 0.01 usdc
    // 0.01 usdc * 10^6 = 10000 usdc in base units
    assert_eq!(fee_in_lamports, 10050, "Fee in lamports should be 10050");
    assert_eq!(fee_in_token, 10050.0, "Fee in token should be 10050");
}

/// Test estimateTransactionFee with invalid mint address
#[tokio::test]
async fn test_estimate_transaction_fee_with_invalid_mint_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "estimateTransactionFee",
            rpc_params![test_tx, "invalid_mint_address"],
        )
        .await;

    assert!(result.is_err(), "Expected error for invalid mint address");
}

/// Test estimateTransactionFee without payment instruction
#[tokio::test]
async fn test_estimate_transaction_fee_without_payment_instruction_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee with token");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");

    let fee_in_lamports = response["fee_in_lamports"].as_u64().unwrap();

    // Fee in lamport is 10000 + payment instruction fee (so 10050)
    assert_eq!(fee_in_lamports, 10050, "Fee in lamports should be 10000, got {fee_in_lamports}");
}

// NOTE: Lookup table is properly tested via mint address (not in transaction accounts, only ATAs)
#[tokio::test]
async fn test_estimate_transaction_fee_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_spl_transfer_checked(
            &usdc_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction with mint in lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee with mint in lookup table");

    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
}

/// Test estimateTransactionFee without fee token parameter with V0 and lookup table
#[tokio::test]
async fn test_estimate_transaction_fee_without_fee_token_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_spl_transfer_checked(
            &usdc_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction with mint in lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee with mint in lookup table");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");

    assert!(
        response["fee_in_token"].is_null(),
        "Expected fee_in_token to be null when not requested"
    );
}

/// Test estimateTransactionFee with fee token parameter with V0 and lookup table
#[tokio::test]
async fn test_estimate_transaction_fee_with_fee_token_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_spl_transfer_checked(
            &usdc_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction with mint in lookup table");

    let usdc_mint_string = usdc_mint.to_string();

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx, usdc_mint_string])
        .await
        .expect("Failed to estimate transaction fee with token and mint in lookup table");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");
    response.assert_has_field("fee_in_token");

    let fee_in_lamports = response["fee_in_lamports"].as_u64().unwrap();
    let fee_in_token = response["fee_in_token"].as_f64().unwrap();

    // Verify the conversion makes sense
    // Mocked price DEFAULT_MOCKED_PRICE is 0.001, so 0.001 SOL per usdc
    // Fee in lamport is 10050
    // 10000 lamports -> 0.00001 SOL -> 0.00001 / 0.001 (sol per usdc) = 0.01 usdc
    // 0.01 usdc * 10^6 = 10000 usdc in base units
    assert_eq!(fee_in_lamports, 10050, "Fee in lamports should be 10050");
    assert_eq!(fee_in_token, 10050.0, "Fee in token should be 10050");
}

/// Comprehensive test covering all fee scenarios: ATA creation, manual token accounts,
/// SPL token operations, compute budget, and priority fees
#[tokio::test]
async fn test_estimate_fee_comprehensive_with_token_accounts_creation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = SenderTestHelper::get_test_sender_keypair();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let usdc_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let recipient1_needs_ata = Pubkey::new_unique();
    let recipient1_2022_needs_ata = Pubkey::new_unique();

    // Manual token accounts (non-ATA)
    let manual_spl_account1 = Keypair::new();

    // Get rent for token accounts
    let token_account_rent = ctx
        .client
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await
        .expect("Failed to get rent exemption amount");

    // Build the comprehensive transaction
    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_create_ata(&usdc_mint, &recipient1_needs_ata)
        .with_create_token2022_ata(&usdc_mint_2022, &recipient1_2022_needs_ata)
        .with_create_and_init_token_account(
            &manual_spl_account1,
            &usdc_mint,
            &sender.pubkey(),
            token_account_rent,
        )
        .build()
        .await
        .expect("Failed to build transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx.clone()])
        .await
        .expect("Failed to estimate transaction fee");

    response.assert_success();
    response.assert_has_field("fee_in_lamports");

    let fee_lamports = response["fee_in_lamports"].as_u64().unwrap();

    // Expected fee breakdown:
    // - Base fee: ~5000 lamports (for signatures)
    // - ATA creation rent: 2_039_280 + 2_039_280 = 4_078_560 lamports (2 ATAs)
    // - Manual token account rent: 2_157_600 lamports (1 manual account, as shown in debug logs)
    let expected_minimum_fee = 5_000 + 4_078_560 + 2_157_600;

    assert!(
        fee_lamports >= expected_minimum_fee,
        "Fee should include all account creations. Got {}, expected at least {}",
        fee_lamports,
        expected_minimum_fee
    );

    assert!(
        fee_lamports < expected_minimum_fee + 50_000,
        "Fee shouldn't be excessively high. Got {}, expected max {}",
        fee_lamports,
        expected_minimum_fee + 50_000
    );
}
