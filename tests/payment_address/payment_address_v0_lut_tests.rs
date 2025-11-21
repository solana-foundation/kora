// NOTE: Lookup table is tested via mint address (not in transaction accounts, only ATAs)
use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;
use std::str::FromStr;

#[tokio::test]
async fn test_sign_transaction_if_paid_with_payment_address_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let encoded_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &test_mint,
            &sender.pubkey(),
            &payment_address,
            get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create signed V0 transaction with mint in lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![encoded_tx])
        .await
        .expect("Failed to sign V0 transaction with mint in lookup table");

    response.assert_success();
    response.assert_has_field("signed_transaction");
}

#[tokio::test]
async fn test_sign_transaction_if_paid_with_wrong_destination_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let wrong_destination = Keypair::new(); // Random wrong destination
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let fee_payer_keypair = FeePayerTestHelper::get_fee_payer_keypair();

    // Create ATA for the wrong destination
    let create_wrong_ata_idempotent_ix = create_associated_token_account_idempotent(
        &fee_payer_keypair.pubkey(),
        &wrong_destination.pubkey(),
        &test_mint,
        &spl_token_interface::id(),
    );

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let encoded_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_instruction(create_wrong_ata_idempotent_ix)
        .with_spl_transfer_checked(
            &test_mint,
            &sender.pubkey(),
            &wrong_destination.pubkey(),
            get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create signed V0 transaction with mint in lookup table");

    let response: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![encoded_tx]).await;

    assert!(response.is_err(), "Expected payment validation to fail for wrong destination");
}
