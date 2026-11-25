use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use spl_associated_token_account_interface::address::get_associated_token_address;
use std::str::FromStr;

async fn sign_payment_into(
    ctx: &TestContext,
    close_authority: Option<&Pubkey>,
) -> anyhow::Result<serde_json::Value> {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let creator = create_funded_wallet(ctx).await;
    let destination = create_token_account_at_keypair_address(
        ctx,
        &creator,
        &test_mint,
        &payment_address,
        close_authority,
    )
    .await;

    let payment_ix = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::id(),
        &get_associated_token_address(&sender.pubkey(), &test_mint),
        &test_mint,
        &destination.pubkey(),
        &sender.pubkey(),
        &[],
        get_fee_for_default_transaction_in_usdc(),
        6,
    )
    .unwrap();

    let encoded_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_instruction(payment_ix)
        .build()
        .await
        .expect("Failed to create payment transaction");

    ctx.rpc_call("signTransaction", rpc_params![encoded_tx]).await
}

#[tokio::test]
async fn test_payment_into_non_ata_payment_address_account_accepted() {
    let ctx = crate::ctx().await;
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();

    for close_authority in [None, Some(&payment_address)] {
        let response = sign_payment_into(&ctx, close_authority).await.unwrap_or_else(|error| {
            panic!(
                "Expected payment with close authority {close_authority:?} to be signed: {error}"
            )
        });
        response.assert_success();
        response.assert_has_field("signed_transaction");
    }
}

#[tokio::test]
async fn test_payment_into_payment_address_account_closable_by_fee_payer_rejected() {
    let ctx = crate::ctx().await;
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    match sign_payment_into(&ctx, Some(&fee_payer)).await {
        Err(error) => error.assert_contains_message("Insufficient token payment"),
        Ok(_) => panic!("Expected a payment into an account the fee payer can close to be ignored"),
    }
}
