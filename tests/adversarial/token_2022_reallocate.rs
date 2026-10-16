// Token-2022 Reallocate with the fee payer as `payer` (kora-test.toml sets
// `fee_payer_policy.token_2022.allow_reallocate = true`).
//
// Token-2022 funds the rent delta of a Reallocate through a System Transfer CPI from `payer`.
// These tests prove against the real program that Kora sees that CPI during simulation, so the
// rent counts toward the fee payer outflow: it is priced into the fee and capped by
// `max_allowed_lamports` (1_000_000 in kora-test.toml).

use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account_interface::{
    address::get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use spl_token_2022_interface::{
    extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
    instruction as token_2022_instruction,
    state::Account,
};

const MAX_ALLOWED_LAMPORTS: u64 = 1_000_000;

/// Creates a Token-2022 ATA for a fresh owner and returns the owner, the ATA, and the rent delta
/// that growing it to also fit `extensions` costs.
async fn create_token_account(
    ctx: &TestContext,
    setup: &TestAccountSetup,
    extensions: &[ExtensionType],
) -> (Keypair, Pubkey, u64) {
    let owner = Keypair::new();
    let mint = setup.usdc_mint_2022.pubkey();
    let token_account = get_associated_token_address_with_program_id(
        &owner.pubkey(),
        &mint,
        &spl_token_2022_interface::id(),
    );
    let create_ata_ix = create_associated_token_account_idempotent(
        &setup.sender_keypair.pubkey(),
        &owner.pubkey(),
        &mint,
        &spl_token_2022_interface::id(),
    );

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup token account");

    let account =
        ctx.rpc_client().get_account(&token_account).await.expect("Failed to fetch token account");
    let mut grown_extensions = StateWithExtensions::<Account>::unpack(&account.data)
        .expect("Failed to unpack token account")
        .get_extension_types()
        .expect("Failed to read token account extensions");
    grown_extensions.extend_from_slice(extensions);
    let grown_len = ExtensionType::try_calculate_account_len::<Account>(&grown_extensions)
        .expect("Failed to calculate grown account length");
    let grown_rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(grown_len)
        .await
        .expect("Failed to get rent");

    (owner, token_account, grown_rent.saturating_sub(account.lamports))
}

fn reallocate_ix(
    token_account: &Pubkey,
    fee_payer: &Pubkey,
    owner: &Pubkey,
    extensions: &[ExtensionType],
) -> Instruction {
    token_2022_instruction::reallocate(
        &spl_token_2022_interface::id(),
        token_account,
        fee_payer,
        owner,
        &[],
        extensions,
    )
    .expect("Failed to create reallocate ix")
}

#[tokio::test]
async fn test_reallocate_rent_funded_by_fee_payer_is_charged_as_outflow() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = setup.sender_keypair.insecure_clone();
    let usdc_mint = setup.usdc_mint.pubkey();
    let extensions = [ExtensionType::CpiGuard];

    let (owner, token_account, rent_delta) = create_token_account(&ctx, &setup, &extensions).await;
    assert!(rent_delta > 0, "Growing the account must cost rent");
    assert!(rent_delta < MAX_ALLOWED_LAMPORTS, "Rent delta must fit max_allowed_lamports");

    let build_tx = |payment: u64| {
        ctx.transaction_builder()
            .with_fee_payer(fee_payer)
            .with_signer(&sender)
            .with_signer(&owner)
            .with_spl_payment(&usdc_mint, &sender.pubkey(), &fee_payer, payment)
            .with_instruction(reallocate_ix(
                &token_account,
                &fee_payer,
                &owner.pubkey(),
                &extensions,
            ))
            .build()
    };

    let estimation_tx = build_tx(1).await.expect("Failed to build estimation transaction");
    let fee_estimate: serde_json::Value = ctx
        .rpc_call("estimateTransactionFee", rpc_params![estimation_tx, usdc_mint.to_string()])
        .await
        .expect("Failed to estimate transaction fee");
    fee_estimate.assert_success();

    let fee_in_lamports =
        fee_estimate["fee_in_lamports"].as_u64().expect("Expected fee_in_lamports");
    assert!(
        fee_in_lamports >= rent_delta,
        "Estimated fee {fee_in_lamports} must include the Reallocate rent delta {rent_delta}"
    );
    let required_usdc = fee_estimate["fee_in_token"].as_u64().expect("Expected fee_in_token");

    let underpaid_tx = build_tx(get_fee_for_default_transaction_in_usdc())
        .await
        .expect("Failed to build underpaid transaction");
    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![underpaid_tx]).await;
    match result {
        Err(error) => error.assert_contains_message("Insufficient token payment"),
        Ok(_) => panic!("Expected payment covering only the base fee to be rejected"),
    }

    let paid_tx = build_tx(required_usdc).await.expect("Failed to build paid transaction");
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![paid_tx])
        .await
        .expect("Expected Reallocate funded by the fee payer to be signed when paid for");
    assert!(response["signed_transaction"].as_str().is_some());
}

#[tokio::test]
async fn test_reallocate_rent_above_max_allowed_lamports_rejected() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let sender = setup.sender_keypair.insecure_clone();
    let extensions = [ExtensionType::ConfidentialTransferAccount];

    let (owner, token_account, rent_delta) = create_token_account(&ctx, &setup, &extensions).await;
    assert!(
        rent_delta > MAX_ALLOWED_LAMPORTS,
        "Rent delta {rent_delta} must exceed max_allowed_lamports for this test"
    );

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_signer(&owner)
        .with_spl_payment(&setup.usdc_mint.pubkey(), &sender.pubkey(), &fee_payer, 5_000_000)
        .with_instruction(reallocate_ix(&token_account, &fee_payer, &owner.pubkey(), &extensions))
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![tx]).await;
    match result {
        Err(error) => error.assert_contains_message(&format!(
            "Total transfer amount {rent_delta} exceeds maximum allowed {MAX_ALLOWED_LAMPORTS}"
        )),
        Ok(_) => panic!("Expected Reallocate rent above max_allowed_lamports to be rejected"),
    }
}
