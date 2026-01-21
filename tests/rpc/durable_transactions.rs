use crate::common::*;
use jsonrpsee::rpc_params;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use solana_system_interface::instruction::create_nonce_account;

#[tokio::test]
async fn test_durable_transaction_rejected() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    // Create nonce account
    let nonce_account = Keypair::new();
    let nonce_authority = sender.pubkey();

    // Get rent for nonce account (80 bytes)
    let rent =
        rpc_client.get_minimum_balance_for_rent_exemption(80).await.expect("Failed to get rent");

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

    rpc_client
        .send_and_confirm_transaction(&create_nonce_tx)
        .await
        .expect("Failed to create nonce account");

    // Build durable nonce transaction
    let durable_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .build_with_nonce(&nonce_account.pubkey())
        .await
        .expect("Failed to build durable nonce transaction");

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
