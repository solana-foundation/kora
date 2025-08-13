use jsonrpsee::{core::client::ClientT, rpc_params};
use kora_lib::transaction::VersionedTransactionUtilExt;
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_message::{v0, Message, VersionedMessage};
use solana_sdk::{signature::Signer, transaction::VersionedTransaction};
use solana_system_interface::instruction::transfer;
use testing_utils::*;

#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_legacy() {
    let rpc_client = get_rpc_client().await;
    let client = get_test_client().await;
    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();

    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(300_000),
        ComputeBudgetInstruction::set_compute_unit_price(50_000),
        transfer(&sender.pubkey(), &recipient, 1_000_000),
    ];

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&sender.pubkey()),
        &blockhash.0,
    ));
    let transaction = VersionedTransaction::try_new(message, &[&sender]).unwrap();

    let encoded_tx = transaction.encode_b64_transaction().expect("Failed to encode transaction");

    let response: serde_json::Value = client
        .request(
            "estimateTransactionFee",
            rpc_params![encoded_tx, get_test_usdc_mint_pubkey().to_string()],
        )
        .await
        .expect("Failed to estimate transaction fee");

    assert!(response.get("fee_in_lamports").is_some(), "Response should have result field");
    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Fee should include priority fee from compute budget instructions
    // Priority fee calculation: 300_000 * 50_000 / 1_000_000 = 15_000 lamports
    // Plus base transaction fee (5000 for this transaction) = 20_000 lamports total
    // Plus Kora signature fee (5000 for this transaction) = 25_000 lamports total
    assert!(fee == 25_000, "Fee should include compute budget priority fee, got {fee}");

    println!("Successfully calculated fee with compute budget instructions: {fee} lamports");
}

#[tokio::test]
async fn test_estimate_transaction_fee_with_compute_budget_v0() {
    let rpc_client = get_rpc_client().await;
    let client = get_test_client().await;
    let fee_payer = get_fee_payer_keypair();
    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();

    let compute_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);
    let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(25_000);
    let transfer_ix = transfer(&sender.pubkey(), &recipient, 500_000);

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .unwrap();

    let v0_message = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &[compute_limit_ix, compute_price_ix, transfer_ix],
        &[],
        blockhash.0,
    )
    .expect("Failed to compile V0 message");

    let message = VersionedMessage::V0(v0_message);
    let transaction = VersionedTransaction::try_new(message, &[&fee_payer, &sender]).unwrap();

    let encoded_tx = transaction.encode_b64_transaction().expect("Failed to encode transaction");

    let response: serde_json::Value = client
        .request(
            "estimateTransactionFee",
            rpc_params![encoded_tx, get_test_usdc_mint_pubkey().to_string()],
        )
        .await
        .expect("Failed to estimate transaction fee with V0 transaction");

    assert!(response.get("fee_in_lamports").is_some(), "Response should have result field");
    let fee = response["fee_in_lamports"].as_u64().expect("Fee should be a number");

    // Priority fee calculation: 1_000_000 * 25_000 / 1_000_000 = 25_000 lamports
    // Plus base transaction fee (2 signatures) (10000 for this transaction) = 35_000 lamports total
    // We don't include the Kora signature EXTRA fee because the fee payer is already Kora and added as a signer
    assert!(fee == 35_000, "Fee should include V0 compute budget priority fee, got {fee}");

    println!("Successfully calculated fee with V0 compute budget instructions: {fee} lamports");
}
