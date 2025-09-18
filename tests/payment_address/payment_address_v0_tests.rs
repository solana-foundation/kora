use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::{
    token::{TokenInterface, TokenProgram},
    transaction::{TransactionUtil, VersionedTransactionOps},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use std::str::FromStr;
use tests::common::helpers::get_fee_for_default_transaction_in_usdc;

#[tokio::test]
async fn test_sign_transaction_if_paid_with_payment_address_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let payment_address_token_account = get_associated_token_address(&payment_address, &test_mint);

    let token_interface = TokenProgram::new();
    let fee_payer_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            get_fee_for_default_transaction_in_usdc(),
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    // Use TransactionBuilder with V0 format (no lookup tables)
    let encoded_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_instruction(fee_payer_instruction)
        .build()
        .await
        .expect("Failed to create signed V0 transaction");

    // Call signTransactionIfPaid endpoint - should succeed when payment goes to correct address
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![encoded_tx])
        .await
        .expect("Failed to sign V0 transaction");

    response.assert_success();
    response.assert_has_field("signed_transaction");
}

#[tokio::test]
async fn test_sign_transaction_if_paid_with_wrong_destination_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let wrong_destination = Keypair::new(); // Random wrong destination
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create a transfer to the WRONG destination (not the payment address)
    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let wrong_dest_ata = get_associated_token_address(&wrong_destination.pubkey(), &test_mint);

    let create_wrong_ata_idempotent_ix = create_associated_token_account_idempotent(
        &fee_payer.pubkey(),
        &wrong_destination.pubkey(),
        &test_mint,
        &spl_token::id(),
    );

    let token_interface = TokenProgram::new();
    let fee_payer_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &wrong_dest_ata,
            &sender.pubkey(),
            get_fee_for_default_transaction_in_usdc(),
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    // Use TransactionBuilder with V0 format (no lookup tables)
    let encoded_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_instruction(create_wrong_ata_idempotent_ix)
        .with_instruction(fee_payer_instruction)
        .build()
        .await
        .expect("Failed to create signed V0 transaction");

    // Call signTransactionIfPaid endpoint - should fail when payment goes to wrong address
    let response: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransactionIfPaid", rpc_params![encoded_tx]).await;

    assert!(response.is_err(), "Expected payment validation to fail for wrong destination");
}
