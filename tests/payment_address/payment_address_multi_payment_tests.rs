use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::token::{spl_token::TokenProgram, TokenInterface};
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use spl_associated_token_account_interface::address::get_associated_token_address;
use std::str::FromStr;

#[tokio::test]
async fn test_sign_transaction_if_paid_with_multiple_payments_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let payment_address_token_account = get_associated_token_address(&payment_address, &test_mint);

    let token_interface = TokenProgram::new();

    let required_fee = get_fee_for_default_transaction_in_usdc();
    let payment_1 = required_fee / 2;
    let payment_2 = required_fee - payment_1 + 10;

    let payment_instruction_1 = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            payment_1,
        )
        .unwrap();

    let payment_instruction_2 = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            payment_2,
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let encoded_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_instruction(payment_instruction_1)
        .with_instruction(payment_instruction_2)
        .build()
        .await
        .expect("Failed to create signed legacy transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![encoded_tx])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();
    response.assert_has_field("signed_transaction");
}

#[tokio::test]
async fn test_sign_transaction_if_paid_with_multiple_payments_insufficient_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let payment_address_token_account = get_associated_token_address(&payment_address, &test_mint);

    let token_interface = TokenProgram::new();

    let required_fee = get_fee_for_default_transaction_in_usdc();
    let payment_1 = required_fee / 3;
    let payment_2 = required_fee / 3;

    let payment_instruction_1 = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            payment_1,
        )
        .unwrap();

    let payment_instruction_2 = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            payment_2,
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let encoded_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_instruction(payment_instruction_1)
        .with_instruction(payment_instruction_2)
        .build()
        .await
        .expect("Failed to create signed legacy transaction");

    let response: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![encoded_tx]).await;

    assert!(response.is_err(), "Should fail with insufficient payment");
}

#[tokio::test]
async fn test_sign_transaction_if_paid_with_multiple_sources_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let test_mint2 = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();

    let required_fee = get_fee_for_default_transaction_in_usdc();
    let payment_amount = required_fee / 2;
    // We need to add 50 lamports, because on that mint for 2022 the transfer fee config is 1%
    let payment_amount_2 = payment_amount + 50;

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let encoded_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &test_mint,
            &sender.pubkey(),
            &payment_address,
            payment_amount,
            6,
        )
        .with_spl_token_2022_transfer_checked(
            &test_mint2,
            &sender.pubkey(),
            &payment_address,
            payment_amount_2,
            6,
        )
        .build()
        .await
        .expect("Failed to create signed legacy transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![encoded_tx])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();
    response.assert_has_field("signed_transaction");
}
