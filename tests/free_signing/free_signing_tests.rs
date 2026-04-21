use crate::common::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::{
    instruction::AccountMeta, pubkey::Pubkey, signature::Signer, transaction::Transaction,
};
use std::str::FromStr;

async fn build_transfer_hook_transaction_for_free_signing(
    ctx: &TestContext,
    amount: u64,
    use_transfer_fee_extension: bool,
) -> anyhow::Result<String> {
    let rpc_client = ctx.rpc_client();
    let hook_program_id =
        Pubkey::from_str(TRANSFER_HOOK_PROGRAM_ID).expect("Invalid transfer hook program ID");

    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let transfer_hook_mint_keypair = USDCMint2022TestHelper::get_test_transfer_hook_mint_keypair();

    ExtensionHelpers::create_mint_with_transfer_hook(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair,
        &hook_program_id,
    )
    .await?;

    let sender_ata = spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        &sender.pubkey(),
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022_interface::id(),
    );

    let recipient_ata = spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        &recipient,
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022_interface::id(),
    );

    let create_sender_ata = spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
        &fee_payer.pubkey(),
        &sender.pubkey(),
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022_interface::id(),
    );

    let create_recipient_ata = spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
        &fee_payer.pubkey(),
        &recipient,
        &transfer_hook_mint_keypair.pubkey(),
        &spl_token_2022_interface::id(),
    );

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let create_atas_tx = Transaction::new_signed_with_payer(
        &[create_sender_ata, create_recipient_ata],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    rpc_client.send_and_confirm_transaction(&create_atas_tx).await?;

    ExtensionHelpers::mint_tokens_to_account(
        rpc_client,
        &fee_payer,
        &transfer_hook_mint_keypair.pubkey(),
        &sender_ata,
        &fee_payer,
        Some(1_000_000),
    )
    .await?;

    let mut transfer_instruction = if use_transfer_fee_extension {
        spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee(
            &spl_token_2022_interface::id(),
            &sender_ata,
            &transfer_hook_mint_keypair.pubkey(),
            &recipient_ata,
            &sender.pubkey(),
            &[],
            amount,
            6,
            0,
        )?
    } else {
        spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &sender_ata,
            &transfer_hook_mint_keypair.pubkey(),
            &recipient_ata,
            &sender.pubkey(),
            &[],
            amount,
            6,
        )?
    };

    let extra_account_metas_address = spl_transfer_hook_interface::get_extra_account_metas_address(
        &transfer_hook_mint_keypair.pubkey(),
        &hook_program_id,
    );

    transfer_instruction
        .accounts
        .push(AccountMeta::new_readonly(extra_account_metas_address, false));
    transfer_instruction.accounts.push(AccountMeta::new_readonly(hook_program_id, false));

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer, &sender],
        recent_blockhash,
    );

    let serialized = bincode::serialize(&transaction)?;
    Ok(STANDARD.encode(serialized))
}

#[tokio::test]
async fn test_sign_transaction_rejects_mutable_transfer_hook_in_free_mode() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = build_transfer_hook_transaction_for_free_signing(&ctx, 10, false)
        .await
        .expect("Failed to create transfer-hook transaction");

    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransaction", rpc_params![test_tx]).await;

    assert!(result.is_err(), "Expected delayed signing to reject mutable transfer-hook authority",);

    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Mutable transfer-hook authority found on mint account"),
        "Expected mutable transfer-hook authority error, got: {error}",
    );
}

#[tokio::test]
async fn test_sign_and_send_transaction_allows_mutable_transfer_hook_in_free_mode() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = build_transfer_hook_transaction_for_free_signing(&ctx, 10, false)
        .await
        .expect("Failed to create transfer-hook transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signAndSendTransaction", rpc_params![test_tx])
        .await
        .expect("Immediate sign-and-send should allow mutable transfer-hook authority");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_transaction_rejects_mutable_transfer_hook_transfer_checked_with_fee_in_free_mode(
) {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = build_transfer_hook_transaction_for_free_signing(&ctx, 10, true)
        .await
        .expect("Failed to create transfer-hook transaction");

    let result: Result<serde_json::Value, anyhow::Error> =
        ctx.rpc_call("signTransaction", rpc_params![test_tx]).await;

    assert!(result.is_err(), "Expected delayed signing to reject mutable transfer-hook authority",);

    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Mutable transfer-hook authority found on mint account"),
        "Expected mutable transfer-hook authority error, got: {error}",
    );
}

#[tokio::test]
async fn test_sign_transaction_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_transaction_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 test transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction simulation failed");
}

#[tokio::test]
async fn test_sign_transaction_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 test transaction with lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 transaction with lookup table");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction with lookup table");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction with lookup table simulation failed");
}

#[tokio::test]
async fn test_sign_spl_transaction_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_transfer(&sender.pubkey(), &RecipientTestHelper::get_recipient_pubkey(), 10)
        .build()
        .await
        .expect("Failed to create signed test SPL transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

#[tokio::test]
async fn test_sign_spl_transaction_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let test_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 signed test SPL transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 SPL transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 SPL transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 SPL transaction simulation failed");
}

#[tokio::test]
async fn test_sign_spl_transaction_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            10,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 signed test SPL transaction with lookup table");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![test_tx])
        .await
        .expect("Failed to sign V0 SPL transaction with lookup table");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    let simulated_tx = ctx
        .rpc_client()
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 SPL transaction with lookup table");

    assert!(
        simulated_tx.value.err.is_none(),
        "V0 SPL transaction with lookup table simulation failed"
    );
}
