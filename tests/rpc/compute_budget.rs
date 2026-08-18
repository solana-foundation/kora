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

/// V1 transactions carry the priority fee as flat lamports in the transaction
/// config instead of ComputeBudget instructions; Kora captures it through
/// getFeeForMessage, so the estimate must include it exactly.
#[tokio::test]
async fn test_estimate_transaction_fee_with_v1_config_priority_fee() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let build_tx = |priority_fee: Option<u64>| {
        let mut builder = ctx
            .v1_transaction_builder()
            .with_fee_payer(fee_payer.pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 500_000);
        if let Some(lamports) = priority_fee {
            builder = builder.with_v1_priority_fee(lamports);
        }
        builder.build()
    };

    let tx_without_priority_fee =
        build_tx(None).await.expect("Failed to create V1 transaction without priority fee");
    let tx_with_priority_fee =
        build_tx(Some(25_000)).await.expect("Failed to create V1 transaction with priority fee");

    let response_without: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![tx_without_priority_fee])
        .await
        .expect("Failed to estimate fee for V1 transaction without priority fee");
    let response_with: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![tx_with_priority_fee])
        .await
        .expect("Failed to estimate fee for V1 transaction with priority fee");

    let fee_without = response_without["fee_in_lamports"].as_u64().expect("Fee should be a number");
    let fee_with = response_with["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Base transaction fee (2 signatures) 10_000 + payment instruction fee 50
    // We don't include the Kora signature EXTRA fee because the fee payer is already Kora and added as a signer
    assert!(
        fee_without == 10_050,
        "V1 fee without priority fee should be 10_050, got {fee_without}"
    );

    // Same transaction plus the flat 25_000 lamport priority fee from the V1 config
    assert!(fee_with == 35_050, "Fee should include the V1 config priority fee, got {fee_with}");
}

/// ComputeBudget instructions are inert in V1: the runtime neither parses nor rejects
/// them, so their priority fee must not reach the estimate. The fee has to match the same
/// transaction built without them.
#[tokio::test]
async fn test_estimate_transaction_fee_v1_ignores_compute_budget_instructions() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let test_tx = ctx
        .v1_transaction_builder()
        .with_fee_payer(fee_payer.pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 500_000)
        .with_compute_budget(1_400_000, 50_000)
        .build()
        .await
        .expect("Failed to create V1 transaction with compute budget instructions");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Base transaction fee (2 signatures) 10_000 + payment instruction fee 50, with nothing
    // from the 1_400_000 * 50_000 / 1_000_000 = 70_000 lamports the instructions ask for
    assert!(fee == 10_050, "V1 fee must ignore ComputeBudget instructions, got {fee}");
}

/// When a V1 transaction carries both a config priority fee and ComputeBudget
/// instructions, only the config counts.
#[tokio::test]
async fn test_estimate_transaction_fee_v1_config_priority_fee_wins_over_compute_budget() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let test_tx = ctx
        .v1_transaction_builder()
        .with_fee_payer(fee_payer.pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 500_000)
        .with_compute_budget(1_400_000, 50_000)
        .with_v1_priority_fee(25_000)
        .build()
        .await
        .expect("Failed to create V1 transaction with compute budget instructions");

    let response: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("Failed to estimate transaction fee");

    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Base transaction fee (2 signatures) 10_000 + payment instruction fee 50 + the flat
    // 25_000 lamport config priority fee
    assert!(fee == 35_050, "V1 fee must come from the config priority fee alone, got {fee}");
}

/// max_priority_fee_lamports is 5_000_000 in kora-test.toml; a V1 config priority
/// fee above it must be rejected before signing.
#[tokio::test]
async fn test_sign_transaction_v1_rejects_priority_fee_above_config_cap() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();

    let test_tx = ctx
        .v1_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_v1_priority_fee(6_000_000)
        .with_transfer(&sender.pubkey(), &RecipientTestHelper::get_recipient_pubkey(), 10)
        .build()
        .await
        .expect("Failed to create V1 transaction");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![test_tx]).await;

    let error = result.expect_err("Expected rejection for V1 priority fee above the config cap");
    assert!(
        error.to_string().contains("Priority fee 6000000 exceeds maximum allowed 5000000"),
        "Expected priority fee cap rejection, got: {error}"
    );
}

/// The same cap applies to legacy ComputeBudget priority fees:
/// 1_400_000 CU * 5_000_000 micro-lamports = 7_000_000 lamports, over the cap.
#[tokio::test]
async fn test_sign_transaction_legacy_rejects_priority_fee_above_config_cap() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_transfer(&sender.pubkey(), &RecipientTestHelper::get_recipient_pubkey(), 10)
        .with_compute_budget(1_400_000, 5_000_000)
        .build()
        .await
        .expect("Failed to create test transaction");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![test_tx]).await;

    let error =
        result.expect_err("Expected rejection for legacy priority fee above the config cap");
    assert!(
        error.to_string().contains("Priority fee 7000000 exceeds maximum allowed 5000000"),
        "Expected priority fee cap rejection, got: {error}"
    );
}

/// A V1 priority fee under the cap signs normally when the payment covers
/// base fee plus priority fee.
#[tokio::test]
async fn test_sign_transaction_v1_accepts_priority_fee_under_config_cap() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();

    let priority_fee = 20_000;
    let test_tx = ctx
        .v1_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_v1_priority_fee(priority_fee)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc() + priority_fee,
        )
        .with_transfer(&sender.pubkey(), &RecipientTestHelper::get_recipient_pubkey(), 10)
        .build()
        .await
        .expect("Failed to create V1 transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V1 transaction with priority fee under the cap");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

// NOTE: Lookup table is properly tested via mint address (not in transaction accounts, only ATAs)
#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer.pubkey())
        .with_spl_transfer_checked(
            &usdc_mint,
            &sender.pubkey(),
            &recipient,
            500_000,
            TEST_USDC_MINT_DECIMALS,
        )
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
