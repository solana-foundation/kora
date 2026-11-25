// Payment destinations that can be swapped out (kora-test.toml uses the fee payer as payment
// address and sets `fee_payer_policy.spl_token.allow_close_account = true`,
// `allow_set_authority = true` and `system.allow_transfer = true`).
//
// A legacy Transfer carries no mint, so Kora prices it with the mints of the source and the
// destination as they are before the transaction runs. Whoever can close the destination can
// recreate it at the same address with another mint and owner, inside the transaction or between
// signing and landing. A signature from the fee payer counts as such a closer, since a caller can
// collect it before the account becomes a payment destination. These tests prove that with these
// flags only the fee payer's associated token account counts as a payment destination, and that
// the fee payer never closes a token account it owns: a wrapped SOL account closes with its
// balance, which would hand a payment made into it back to the caller.

use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_program_pack::Pack;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::{create_account, transfer};
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint},
};

const PAYMENT_AMOUNT: u64 = 2_000_000;

async fn create_token_account_ix(
    ctx: &TestContext,
    payer: &Keypair,
    address: &Pubkey,
) -> Instruction {
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)
        .await
        .expect("Failed to get token account rent");
    create_account(
        &payer.pubkey(),
        address,
        rent,
        TokenAccount::LEN as u64,
        &spl_token_interface::id(),
    )
}

fn initialize_account_ix(address: &Pubkey, mint: &Pubkey, owner: &Keypair) -> Instruction {
    token_instruction::initialize_account3(
        &spl_token_interface::id(),
        address,
        mint,
        &owner.pubkey(),
    )
    .expect("Failed to create InitializeAccount3 instruction")
}

fn close_account_ix(address: &Pubkey, owner: &Keypair) -> Instruction {
    token_instruction::close_account(
        &spl_token_interface::id(),
        address,
        &owner.pubkey(),
        &owner.pubkey(),
        &[],
    )
    .expect("Failed to create CloseAccount instruction")
}

async fn sender_payment_tx(
    ctx: &TestContext,
    setup: &TestAccountSetup,
    destination: &Pubkey,
) -> String {
    let mint = setup.usdc_mint.pubkey();
    let payment_ix = token_instruction::transfer_checked(
        &spl_token_interface::id(),
        &get_associated_token_address(&setup.sender_keypair.pubkey(), &mint),
        &mint,
        destination,
        &setup.sender_keypair.pubkey(),
        &[],
        PAYMENT_AMOUNT,
        6,
    )
    .expect("Failed to create TransferChecked instruction");

    ctx.transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&setup.sender_keypair)
        .with_instruction(payment_ix)
        .build()
        .await
        .expect("Failed to create payment transaction")
}

async fn assert_payment_into_ignored(
    ctx: &TestContext,
    setup: &TestAccountSetup,
    destination: &Pubkey,
) {
    let payment_tx = sender_payment_tx(ctx, setup, destination).await;
    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![payment_tx]).await;

    match result {
        Err(error) => error.assert_contains_message("Insufficient token payment"),
        Ok(_) => panic!("Expected the payment into {destination} to be ignored"),
    }
}

#[tokio::test]
async fn test_transfer_between_recreated_accounts_is_not_a_payment() {
    let ctx = crate::ctx().await;
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let allowed_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let attacker = create_funded_wallet(&ctx).await;
    let worthless_mint = Keypair::new();
    let source = Keypair::new();

    let mint_rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .expect("Failed to get mint rent");
    let setup_ixs = [
        create_account(
            &attacker.pubkey(),
            &worthless_mint.pubkey(),
            mint_rent,
            Mint::LEN as u64,
            &spl_token_interface::id(),
        ),
        token_instruction::initialize_mint2(
            &spl_token_interface::id(),
            &worthless_mint.pubkey(),
            &attacker.pubkey(),
            None,
            6,
        )
        .expect("Failed to create InitializeMint2 instruction"),
        create_token_account_ix(&ctx, &attacker, &source.pubkey()).await,
        initialize_account_ix(&source.pubkey(), &allowed_mint, &attacker),
    ];
    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let setup_tx = Transaction::new_signed_with_payer(
        &setup_ixs,
        Some(&attacker.pubkey()),
        &[&attacker, &worthless_mint, &source],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&setup_tx)
        .await
        .expect("Failed to send setup transaction");
    let destination = create_token_account_at_keypair_address(
        &ctx,
        &attacker,
        &allowed_mint,
        &fee_payer_pubkey,
        Some(&attacker.pubkey()),
    )
    .await;

    #[allow(deprecated)]
    let legacy_transfer_ix = token_instruction::transfer(
        &spl_token_interface::id(),
        &source.pubkey(),
        &destination.pubkey(),
        &attacker.pubkey(),
        &[],
        PAYMENT_AMOUNT,
    )
    .expect("Failed to create Transfer instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_signer(&attacker)
        .with_signer(&source)
        .with_signer(&destination)
        .with_instruction(close_account_ix(&source.pubkey(), &attacker))
        .with_instruction(close_account_ix(&destination.pubkey(), &attacker))
        .with_instruction(create_token_account_ix(&ctx, &attacker, &source.pubkey()).await)
        .with_instruction(create_token_account_ix(&ctx, &attacker, &destination.pubkey()).await)
        .with_instruction(initialize_account_ix(
            &source.pubkey(),
            &worthless_mint.pubkey(),
            &attacker,
        ))
        .with_instruction(initialize_account_ix(
            &destination.pubkey(),
            &worthless_mint.pubkey(),
            &attacker,
        ))
        .with_instruction(
            token_instruction::mint_to(
                &spl_token_interface::id(),
                &worthless_mint.pubkey(),
                &source.pubkey(),
                &attacker.pubkey(),
                &[],
                PAYMENT_AMOUNT,
            )
            .expect("Failed to create MintTo instruction"),
        )
        .with_instruction(legacy_transfer_ix)
        .with_instruction(transfer(&fee_payer_pubkey, &attacker.pubkey(), 1_000_000))
        .build()
        .await
        .expect("Failed to create transaction recreating both payment accounts");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => error.assert_contains_message("Insufficient token payment"),
        Ok(_) => panic!("Expected a transfer of the attacker's own mint not to count as payment"),
    }
}

#[tokio::test]
async fn test_payment_into_account_closable_by_attacker_rejected() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let attacker = create_funded_wallet(&ctx).await;
    let destination = create_token_account_at_keypair_address(
        &ctx,
        &attacker,
        &setup.usdc_mint.pubkey(),
        &FeePayerTestHelper::get_fee_payer_pubkey(),
        Some(&attacker.pubkey()),
    )
    .await;

    assert_payment_into_ignored(&ctx, &setup, &destination.pubkey()).await;
}

#[tokio::test]
async fn test_payment_into_account_closable_by_fee_payer_rejected() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let attacker = create_funded_wallet(&ctx).await;
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let destination = create_token_account_at_keypair_address(
        &ctx,
        &attacker,
        &setup.usdc_mint.pubkey(),
        &fee_payer_pubkey,
        Some(&fee_payer_pubkey),
    )
    .await;

    assert_payment_into_ignored(&ctx, &setup, &destination.pubkey()).await;
}

#[tokio::test]
async fn test_payment_into_non_ata_account_rejected_when_fee_payer_can_close() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let attacker = create_funded_wallet(&ctx).await;
    let destination = create_token_account_at_keypair_address(
        &ctx,
        &attacker,
        &setup.usdc_mint.pubkey(),
        &FeePayerTestHelper::get_fee_payer_pubkey(),
        None,
    )
    .await;

    assert_payment_into_ignored(&ctx, &setup, &destination.pubkey()).await;
}

#[tokio::test]
async fn test_payment_into_fee_payer_ata_accepted_when_fee_payer_can_close() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let fee_payer_ata = get_associated_token_address(
        &FeePayerTestHelper::get_fee_payer_pubkey(),
        &setup.usdc_mint.pubkey(),
    );

    let payment_tx = sender_payment_tx(&ctx, &setup, &fee_payer_ata).await;
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![payment_tx])
        .await
        .expect("Expected a payment into the fee payer's associated token account to be signed");
    assert!(response["signed_transaction"].as_str().is_some());
}

#[tokio::test]
async fn test_fee_payer_close_of_own_token_account_rejected() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let creator = create_funded_wallet(&ctx).await;
    let kora_token_account = create_token_account_at_keypair_address(
        &ctx,
        &creator,
        &setup.usdc_mint.pubkey(),
        &fee_payer_pubkey,
        None,
    )
    .await;

    let close_ix = token_instruction::close_account(
        &spl_token_interface::id(),
        &kora_token_account.pubkey(),
        &creator.pubkey(),
        &fee_payer_pubkey,
        &[],
    )
    .expect("Failed to create CloseAccount instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(close_ix)
        .build()
        .await
        .expect("Failed to create transaction closing a fee payer token account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => error.assert_contains_message("owned by Kora"),
        Ok(_) => panic!("Expected the fee payer not to close a token account it owns"),
    }
}
