use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1_000_000)
        .with_compute_budget(300_000, 50_000)
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response.get("fee_in_lamports").is_some(), "Response should have result field");
    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Fee should include priority fee from compute budget instructions
    // Priority fee calculation: 300_000 * 50_000 / 1_000_000 = 15_000 lamports
    // Plus base transaction fee (5000 for this transaction) = 20_000 lamports total
    // Plus Kora signature fee (5000 for this transaction) = 25_000 lamports total
    // Plus payment instruction fee (50 lamports) = 25_050 lamports total
    assert!(fee == 25_050, "Fee should include compute budget priority fee, got {fee}");
}

#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer.pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 500_000)
        .with_compute_budget(1_000_000, 25_000)
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response.get("fee_in_lamports").is_some(), "Response should have result field");
    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Priority fee calculation: 1_000_000 * 25_000 / 1_000_000 = 25_000 lamports
    // Plus base transaction fee (2 signatures) (10000 for this transaction) = 35_000 lamports total
    // Plus payment instruction fee (50 lamports) = 35_050 lamports total
    // We don't include the Kora signature EXTRA fee because the fee payer is already Kora and added as a signer
    assert!(fee == 35_050, "Fee should include V0 compute budget priority fee, got {fee}");
}

// NOTE: Lookup table is properly tested via mint address (not in transaction accounts, only ATAs)
#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Use the transaction lookup table which contains the mint address used for ATA derivation
    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer.pubkey())
        .with_spl_transfer_checked(&usdc_mint, &sender.pubkey(), &recipient, 500_000, 6)
        .with_compute_budget(1_000_000, 25_000)
        .build()
        .await
        .expect("Failed to create V0 transaction with mint in lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee with mint in lookup table");

    assert!(response.get("fee_in_lamports").is_some(), "Response should have result field");
    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Priority fee calculation: 1_000_000 * 25_000 / 1_000_000 = 25_000 lamports
    // Plus base transaction fee (2 signatures) (10000 for this transaction) = 35_000 lamports total
    // Plus payment instruction fee (50 lamports) = 35_050 lamports total
    // We don't include the Kora signature EXTRA fee because the fee payer is already Kora and added as a signer
    assert!(
        fee == 35_050,
        "Fee should include V0 compute budget priority fee with mint in lookup table, got {fee}"
    );
}
