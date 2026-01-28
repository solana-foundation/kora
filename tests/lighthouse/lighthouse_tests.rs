// Lighthouse Integration Tests
//
// Tests verify that lighthouse fee payer protection assertions are properly
// added to transactions when lighthouse is enabled.
//
// NOTE: signAndSendTransaction tests are intentionally omitted because when
// lighthouse modifies a transaction (adding the assertion instruction), any
// existing client signatures become invalid. The signAndSend flow would fail
// at the network level with "signature verification failure".
//
// For production use with lighthouse:
// - Use signTransaction → client signs → client sends
// - OR ensure transactions don't have pre-existing client signatures

use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::VersionedTransaction};
use std::str::FromStr;

const LIGHTHOUSE_PROGRAM_ID: &str = "L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95";

fn verify_lighthouse_assertion_added(
    transaction: &solana_sdk::transaction::VersionedTransaction,
) -> bool {
    let lighthouse_pubkey =
        Pubkey::from_str(LIGHTHOUSE_PROGRAM_ID).expect("Invalid lighthouse program ID");
    let account_keys = transaction.message.static_account_keys();

    let lighthouse_index = account_keys.iter().position(|k| *k == lighthouse_pubkey);

    if lighthouse_index.is_none() {
        return false;
    }

    let lighthouse_idx = lighthouse_index.unwrap() as u8;
    let instructions = transaction.message.instructions();

    instructions.last().is_some_and(|ix| ix.program_id_index == lighthouse_idx)
}

/// Re-sign a versioned transaction with a keypair at a specific position
fn resign_transaction(
    transaction: &mut VersionedTransaction,
    signer: &solana_sdk::signature::Keypair,
) -> Result<(), String> {
    let account_keys = transaction.message.static_account_keys();
    let position = account_keys
        .iter()
        .position(|key| key == &signer.pubkey())
        .ok_or_else(|| format!("Signer {} not found in transaction", signer.pubkey()))?;

    let message_bytes = transaction.message.serialize();
    let signature = signer.sign_message(&message_bytes);
    transaction.signatures[position] = signature;
    Ok(())
}

// **************************************************************************************
// Single Transaction Tests
// **************************************************************************************

#[tokio::test]
async fn test_sign_transaction_with_lighthouse_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let base64_transaction = ctx
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
        .expect("Failed to create transaction");

    let original_tx = TransactionUtil::decode_b64_transaction(&base64_transaction)
        .expect("Failed to decode original transaction");
    let original_ix_count = original_tx.message.instructions().len();

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();

    let signed_tx_b64 = response["signed_transaction"].as_str().unwrap();
    let signed_tx = TransactionUtil::decode_b64_transaction(signed_tx_b64)
        .expect("Failed to decode signed transaction");

    assert_eq!(
        signed_tx.message.instructions().len(),
        original_ix_count + 1,
        "Expected one additional instruction (lighthouse assertion)"
    );

    assert!(
        verify_lighthouse_assertion_added(&signed_tx),
        "Last instruction should be lighthouse program"
    );

    let sim_result =
        rpc_client.simulate_transaction(&signed_tx).await.expect("Failed to simulate transaction");

    assert!(
        sim_result.value.err.is_none(),
        "Transaction simulation with lighthouse assertion should succeed: {:?}",
        sim_result.value.err
    );
}

#[tokio::test]
async fn test_sign_transaction_with_lighthouse_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let base64_transaction = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction");

    let original_tx = TransactionUtil::decode_b64_transaction(&base64_transaction)
        .expect("Failed to decode original V0 transaction");
    let original_ix_count = original_tx.message.instructions().len();

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 transaction");

    response.assert_success();

    let signed_tx_b64 = response["signed_transaction"].as_str().unwrap();
    let signed_tx = TransactionUtil::decode_b64_transaction(signed_tx_b64)
        .expect("Failed to decode signed V0 transaction");

    assert_eq!(
        signed_tx.message.instructions().len(),
        original_ix_count + 1,
        "Expected one additional instruction (lighthouse assertion) for V0"
    );

    assert!(
        verify_lighthouse_assertion_added(&signed_tx),
        "Last instruction should be lighthouse program for V0 transaction"
    );

    let sim_result = rpc_client
        .simulate_transaction(&signed_tx)
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(
        sim_result.value.err.is_none(),
        "V0 transaction simulation with lighthouse assertion should succeed: {:?}",
        sim_result.value.err
    );
}

// **************************************************************************************
// Bundle Tests
// **************************************************************************************

#[tokio::test]
async fn test_sign_bundle_with_lighthouse() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let mut transactions = Vec::new();
    let mut original_ix_counts = Vec::new();

    for i in 0..3 {
        let tx = ctx
            .transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_spl_transfer(
                &token_mint,
                &sender.pubkey(),
                &fee_payer,
                tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            )
            .with_transfer(&sender.pubkey(), &recipient, 10 + i)
            .build()
            .await
            .expect("Failed to create transaction");

        let decoded =
            TransactionUtil::decode_b64_transaction(&tx).expect("Failed to decode transaction");
        original_ix_counts.push(decoded.message.instructions().len());
        transactions.push(tx);
    }

    let response: serde_json::Value =
        ctx.rpc_call("signBundle", rpc_params![transactions]).await.expect("Failed to sign bundle");

    response.assert_success();

    let signed_txs = response["signed_transactions"].as_array().unwrap();
    assert_eq!(signed_txs.len(), 3, "Expected 3 signed transactions");

    for (i, signed_tx_value) in signed_txs.iter().enumerate() {
        let signed_tx = TransactionUtil::decode_b64_transaction(signed_tx_value.as_str().unwrap())
            .expect("Failed to decode signed transaction");

        let is_last = i == signed_txs.len() - 1;

        if is_last {
            assert_eq!(
                signed_tx.message.instructions().len(),
                original_ix_counts[i] + 1,
                "Last transaction should have lighthouse assertion added"
            );
            assert!(
                verify_lighthouse_assertion_added(&signed_tx),
                "Last transaction's last instruction should be lighthouse program"
            );
        } else {
            assert_eq!(
                signed_tx.message.instructions().len(),
                original_ix_counts[i],
                "Non-last transactions should not have lighthouse assertion"
            );
        }
    }
}

// **************************************************************************************
// End-to-End Tests (Full Flow with Client Re-signing)
// **************************************************************************************

/// End-to-end test: client signs → Kora adds lighthouse → client re-signs → simulate succeeds
#[tokio::test]
async fn test_lighthouse_end_to_end_with_client_resign() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Step 1: Build transaction with sender signature
    let base64_transaction = ctx
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
        .expect("Failed to create transaction");

    let original_tx = TransactionUtil::decode_b64_transaction(&base64_transaction)
        .expect("Failed to decode original transaction");
    let original_ix_count = original_tx.message.instructions().len();

    // Step 2: Call signTransaction (Kora adds lighthouse and signs as fee payer)
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();

    let signed_tx_b64 = response["signed_transaction"].as_str().unwrap();
    let mut signed_tx = TransactionUtil::decode_b64_transaction(signed_tx_b64)
        .expect("Failed to decode signed transaction");

    // Verify lighthouse assertion was added
    assert_eq!(
        signed_tx.message.instructions().len(),
        original_ix_count + 1,
        "Expected one additional instruction (lighthouse assertion)"
    );
    assert!(
        verify_lighthouse_assertion_added(&signed_tx),
        "Last instruction should be lighthouse program"
    );

    // Step 3: Client re-signs (required because message changed)
    resign_transaction(&mut signed_tx, &sender).expect("Failed to re-sign transaction");

    // Step 4: Simulate with sig_verify=true to verify both signatures are valid
    let sim_result = rpc_client
        .simulate_transaction_with_config(
            &signed_tx,
            RpcSimulateTransactionConfig {
                sig_verify: true,
                commitment: Some(rpc_client.commitment()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to simulate transaction");

    assert!(
        sim_result.value.err.is_none(),
        "Transaction simulation should succeed after re-signing. Error: {:?}",
        sim_result.value.err
    );
}

/// V0 version of the end-to-end test
#[tokio::test]
async fn test_lighthouse_end_to_end_v0_with_client_resign() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let rpc_client = ctx.rpc_client();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Step 1: Build V0 transaction with sender signature
    let base64_transaction = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction");

    let original_tx = TransactionUtil::decode_b64_transaction(&base64_transaction)
        .expect("Failed to decode original V0 transaction");
    let original_ix_count = original_tx.message.instructions().len();

    // Step 2: Call signTransaction (Kora adds lighthouse and signs as fee payer)
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 transaction");

    response.assert_success();

    let signed_tx_b64 = response["signed_transaction"].as_str().unwrap();
    let mut signed_tx = TransactionUtil::decode_b64_transaction(signed_tx_b64)
        .expect("Failed to decode signed V0 transaction");

    // Verify lighthouse assertion was added
    assert_eq!(
        signed_tx.message.instructions().len(),
        original_ix_count + 1,
        "Expected one additional instruction (lighthouse assertion) for V0"
    );
    assert!(
        verify_lighthouse_assertion_added(&signed_tx),
        "Last instruction should be lighthouse program for V0"
    );

    // Step 3: Client re-signs (required because message changed)
    resign_transaction(&mut signed_tx, &sender).expect("Failed to re-sign V0 transaction");

    // Step 4: Simulate with sig_verify=true to verify both signatures are valid
    let sim_result = rpc_client
        .simulate_transaction_with_config(
            &signed_tx,
            RpcSimulateTransactionConfig {
                sig_verify: true,
                commitment: Some(rpc_client.commitment()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(
        sim_result.value.err.is_none(),
        "V0 transaction simulation should succeed after re-signing. Error: {:?}",
        sim_result.value.err
    );
}
