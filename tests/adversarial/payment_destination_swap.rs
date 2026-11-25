// Payment destinations that someone other than Kora can close (kora-test.toml sets
// `fee_payer_policy.spl_token.allow_close_account = true` and `system.allow_transfer = true`).
//
// A legacy Transfer carries no mint, so Kora prices it with the mints of the source and the
// destination as they are before the transaction runs. Whoever can close the destination can
// recreate it at the same address with another mint and owner, inside the transaction or between
// signing and landing. These tests prove that a payment counts only into an account that only
// Kora can close, and that Kora does not close a token account it owns, since the caller may hold
// that address's keypair.

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
    instruction::{self as token_instruction, AuthorityType},
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

async fn send_as(ctx: &TestContext, payer: &Keypair, ixs: &[Instruction], signers: &[&Keypair]) {
    let mut all_signers = vec![payer];
    all_signers.extend_from_slice(signers);
    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        ixs,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to send setup transaction");
}

/// Creates a token account of `mint` at a fresh keypair address, owned by the fee payer. With
/// `creator_keeps_close_authority` the creator stays its close authority.
async fn create_kora_owned_token_account(
    ctx: &TestContext,
    creator: &Keypair,
    mint: &Pubkey,
    creator_keeps_close_authority: bool,
) -> Keypair {
    let account = Keypair::new();
    let mut ixs = vec![
        create_token_account_ix(ctx, creator, &account.pubkey()).await,
        initialize_account_ix(&account.pubkey(), mint, creator),
    ];
    if creator_keeps_close_authority {
        ixs.push(
            token_instruction::set_authority(
                &spl_token_interface::id(),
                &account.pubkey(),
                Some(&creator.pubkey()),
                AuthorityType::CloseAccount,
                &creator.pubkey(),
                &[],
            )
            .expect("Failed to create SetAuthority instruction"),
        );
    }
    ixs.push(
        token_instruction::set_authority(
            &spl_token_interface::id(),
            &account.pubkey(),
            Some(&FeePayerTestHelper::get_fee_payer_pubkey()),
            AuthorityType::AccountOwner,
            &creator.pubkey(),
            &[],
        )
        .expect("Failed to create SetAuthority instruction"),
    );
    send_as(ctx, creator, &ixs, &[&account]).await;
    account
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
    let setup_ixs = vec![
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
    send_as(&ctx, &attacker, &setup_ixs, &[&worthless_mint, &source]).await;
    let destination = create_kora_owned_token_account(&ctx, &attacker, &allowed_mint, true).await;

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
    let destination =
        create_kora_owned_token_account(&ctx, &attacker, &setup.usdc_mint.pubkey(), true).await;

    let payment_tx = sender_payment_tx(&ctx, &setup, &destination.pubkey()).await;
    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![payment_tx]).await;

    match result {
        Err(error) => error.assert_contains_message("Insufficient token payment"),
        Ok(_) => panic!("Expected a payment into an account the attacker can close to be ignored"),
    }
}

#[tokio::test]
async fn test_payment_into_non_ata_account_only_kora_can_close_accepted() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let creator = create_funded_wallet(&ctx).await;
    let destination =
        create_kora_owned_token_account(&ctx, &creator, &setup.usdc_mint.pubkey(), false).await;

    let payment_tx = sender_payment_tx(&ctx, &setup, &destination.pubkey()).await;
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![payment_tx])
        .await
        .expect("Expected a payment into a Kora-owned account only Kora can close to be signed");
    assert!(response["signed_transaction"].as_str().is_some());
}

#[tokio::test]
async fn test_fee_payer_close_of_own_token_account_rejected() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let attacker = create_funded_wallet(&ctx).await;
    let kora_token_account =
        create_kora_owned_token_account(&ctx, &attacker, &setup.usdc_mint.pubkey(), false).await;

    let close_ix = token_instruction::close_account(
        &spl_token_interface::id(),
        &kora_token_account.pubkey(),
        &fee_payer_pubkey,
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
        .expect("Failed to create transaction closing a Kora token account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => error.assert_contains_message("owned by Kora"),
        Ok(_) => panic!("Expected the fee payer not to close a token account it owns"),
    }
}
