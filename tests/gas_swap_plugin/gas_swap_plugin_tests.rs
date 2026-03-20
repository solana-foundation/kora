// Gas Swap Plugin Integration Tests
//
// Tests verify that the GasSwap plugin correctly enforces the gas-swap transaction shape:
//   ix[0] = SplTokenTransfer  (user pays fee in tokens)
//   ix[1] = SystemTransfer    (fee payer sends SOL to user)
//
// The plugin runs on signTransaction and signAndSendTransaction.

use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::signature::Signer;

// **************************************************************************************
// Valid shape tests
// **************************************************************************************

/// Valid gas-swap shape: SplTokenTransfer + SystemTransfer → should succeed
#[tokio::test]
async fn test_gas_swap_plugin_accepts_valid_shape() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let base64_transaction = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        // Payment must cover: base_fee (5000) + kora_sig_fee (5000) + fee_payer_outflow (10_000) = 20_000
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            2 * tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&fee_payer, &sender.pubkey(), 10_000)
        .build()
        .await
        .expect("Failed to create gas-swap transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![base64_transaction])
        .await
        .expect("signTransaction should succeed for valid gas-swap shape");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

// **************************************************************************************
// Rejection tests
// **************************************************************************************

/// 4 instructions: compute_budget x2 + SplTokenTransfer + SystemTransfer → should fail
#[tokio::test]
async fn test_gas_swap_plugin_rejects_extra_instruction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let base64_transaction = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_compute_budget(200_000, 1)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            tests::common::helpers::get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(&fee_payer, &sender.pubkey(), 10_000)
        .build()
        .await
        .expect("Failed to create transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![base64_transaction])
        .await;

    match result {
        Err(error) => {
            error.assert_contains_message("GasSwap plugin requires exactly 2 instructions")
        }
        Ok(_) => panic!("Expected GasSwap plugin rejection for extra instruction"),
    }
}

/// Only SystemTransfer (1 instruction, not 2) → should fail
#[tokio::test]
async fn test_gas_swap_plugin_rejects_missing_token_transfer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();

    let base64_transaction = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        // No .with_signer(&sender) — sender is only the recipient, not a required signer
        .with_transfer(&fee_payer, &sender.pubkey(), 10_000)
        .build()
        .await
        .expect("Failed to create transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![base64_transaction])
        .await;

    match result {
        Err(error) => {
            error.assert_contains_message("GasSwap plugin requires exactly 2 instructions")
        }
        Ok(_) => panic!("Expected GasSwap plugin rejection for missing token transfer"),
    }
}

/// Only SplTokenTransfer (1 instruction, not 2) → should fail
#[tokio::test]
async fn test_gas_swap_plugin_rejects_missing_sol_transfer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
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
        .build()
        .await
        .expect("Failed to create transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![base64_transaction])
        .await;

    match result {
        Err(error) => {
            error.assert_contains_message("GasSwap plugin requires exactly 2 instructions")
        }
        Ok(_) => panic!("Expected GasSwap plugin rejection for missing SOL transfer"),
    }
}

/// SplTokenTransfer + SystemAssign (not Transfer) → should fail
#[tokio::test]
async fn test_gas_swap_plugin_rejects_non_transfer_system_instruction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let system_program_id = solana_system_interface::program::ID;

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
        .with_system_assign(&sender.pubkey(), &system_program_id)
        .build()
        .await
        .expect("Failed to create transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![base64_transaction])
        .await;

    match result {
        Err(error) => error.assert_contains_message(
            "GasSwap plugin requires instruction 1 to be SystemInstruction::Transfer",
        ),
        Ok(_) => panic!("Expected GasSwap plugin rejection for non-transfer system instruction"),
    }
}
