use crate::common::*;
use jsonrpsee::rpc_params;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use solana_system_interface::instruction::create_nonce_account;

/// Test that durable transactions (with AdvanceNonceAccount) are rejected by default
#[tokio::test]
async fn test_durable_transaction_rejected() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // User creates their own nonce account
    let nonce_account = Keypair::new();
    let nonce_authority = sender.pubkey();

    // Get rent for nonce account (80 bytes)
    let rent =
        rpc_client.get_minimum_balance_for_rent_exemption(80).await.expect("Failed to get rent");

    // User creates and pays for their own nonce account
    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .expect("Failed to get blockhash")
        .0;

    let create_nonce_ix =
        create_nonce_account(&sender.pubkey(), &nonce_account.pubkey(), &nonce_authority, rent);

    let create_nonce_tx = Transaction::new_signed_with_payer(
        &create_nonce_ix,
        Some(&sender.pubkey()),
        &[&sender, &nonce_account],
        blockhash,
    );

    // Send and wait for confirmation so the nonce account is fully initialized
    rpc_client
        .send_and_confirm_transaction(&create_nonce_tx)
        .await
        .expect("Failed to create nonce account");

    // Create a durable transaction
    let durable_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .with_advance_nonce(&nonce_account.pubkey(), &nonce_authority)
        .build_with_nonce(&nonce_account.pubkey())
        .await
        .expect("Failed to create durable transaction");

    // Attempt to sign - should be rejected because durable transactions are blocked
    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![durable_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Durable transactions (nonce-based) are not allowed");
        }
        Ok(_) => panic!("Expected durable transaction to be rejected"),
    }
}

/// Test that regular transactions (without AdvanceNonceAccount) still work
#[tokio::test]
async fn test_regular_transaction_passes() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create a regular transaction (no nonce)
    let regular_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create regular transaction");

    // Sign should succeed
    let result: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![regular_tx])
        .await
        .expect("Expected regular transaction to be signed");

    result.assert_success();
    assert!(
        result["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}
