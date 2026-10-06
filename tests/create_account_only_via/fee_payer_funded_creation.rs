use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
    signature::Keypair,
    signer::Signer,
};
use solana_system_interface::{
    instruction::{create_account, transfer},
    program::ID as SYSTEM_PROGRAM_ID,
};
use std::str::FromStr;

const ONLY_VIA_ERROR: &str =
    "Fee payer may fund account creation only inside a CPI from one of create_account_only_via";

fn via_forwarder(target: Instruction) -> Instruction {
    let mut accounts = vec![AccountMeta::new_readonly(target.program_id, false)];
    accounts.extend(target.accounts);
    Instruction {
        program_id: Pubkey::from_str(TRANSFER_HOOK_PROGRAM_ID).unwrap(),
        accounts,
        data: target.data,
    }
}

/// A forwarder call that creates nothing: it relays a 1-lamport transfer paid by the test sender.
fn forwarder_noop() -> Instruction {
    let sender = SenderTestHelper::get_test_sender_keypair().pubkey();
    via_forwarder(transfer(&sender, &RecipientTestHelper::get_recipient_pubkey(), 1))
}

fn fee_payer_creates(new_account: &Pubkey) -> Instruction {
    create_account(
        &FeePayerTestHelper::get_fee_payer_pubkey(),
        new_account,
        Rent::default().minimum_balance(0),
        0,
        &SYSTEM_PROGRAM_ID,
    )
}

#[tokio::test]
async fn test_top_level_create_account_funded_by_fee_payer_rejected() {
    let ctx = crate::ctx().await;
    let new_account = Keypair::new();

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(forwarder_noop())
        .with_instruction(fee_payer_creates(&new_account.pubkey()))
        .with_signer(&SenderTestHelper::get_test_sender_keypair())
        .with_signer(&new_account)
        .build()
        .await
        .expect("Failed to build transaction");

    let error = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![tx])
        .await
        .expect_err("top-level CreateAccount funded by the fee payer must be rejected");
    error.assert_contains_message(ONLY_VIA_ERROR);
    error.assert_contains_message(
        "instruction 1 ('System Create Account') is a top-level instruction",
    );
}

#[tokio::test]
async fn test_create_account_inside_listed_program_cpi_accepted() {
    let ctx = crate::ctx().await;
    let new_account = Keypair::new();

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(via_forwarder(fee_payer_creates(&new_account.pubkey())))
        .with_signer(&new_account)
        .build()
        .await
        .expect("Failed to build transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![tx])
        .await
        .expect("CreateAccount issued by the listed program's CPI must be accepted");
    response.assert_success();
    assert!(response["signed_transaction"].as_str().is_some());
}

#[tokio::test]
async fn test_sibling_top_level_create_account_rejected_when_listed_program_creates_too() {
    let ctx = crate::ctx().await;
    let via_cpi = Keypair::new();
    let top_level = Keypair::new();

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(via_forwarder(fee_payer_creates(&via_cpi.pubkey())))
        .with_instruction(fee_payer_creates(&top_level.pubkey()))
        .with_signer(&via_cpi)
        .with_signer(&top_level)
        .build()
        .await
        .expect("Failed to build transaction");

    let error = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![tx])
        .await
        .expect_err("a top-level creation next to a legitimate CPI creation must be rejected");
    error.assert_contains_message(
        "instruction 1 ('System Create Account') is a top-level instruction",
    );
}

#[tokio::test]
async fn test_top_level_ata_create_funded_by_fee_payer_rejected() {
    let ctx = crate::ctx().await;

    let tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(forwarder_noop())
        .with_create_ata(&USDCMintTestHelper::get_test_usdc_mint_pubkey(), &Pubkey::new_unique())
        .with_signer(&SenderTestHelper::get_test_sender_keypair())
        .build()
        .await
        .expect("Failed to build transaction");

    let error = ctx
        .rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![tx])
        .await
        .expect_err("top-level ATA Create funded by the fee payer must be rejected");
    error.assert_contains_message(
        "instruction 1 ('Associated Token Account Create') is a top-level instruction",
    );
}

#[tokio::test]
async fn test_get_config_reports_create_account_only_via() {
    let ctx = crate::ctx().await;

    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("getConfig failed");
    assert_eq!(
        response["validation_config"]["fee_payer_policy"]["system"]["create_account_only_via"],
        serde_json::json!([TRANSFER_HOOK_PROGRAM_ID])
    );
}
