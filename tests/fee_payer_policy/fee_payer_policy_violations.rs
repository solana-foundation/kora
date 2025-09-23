use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{
    program_pack::Pack, signature::Keypair, signer::Signer, transaction::Transaction,
};
use solana_system_interface::instruction::{create_account, transfer};
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token::instruction as token_instruction;
use spl_token_2022::instruction as token_2022_instruction;

#[tokio::test]
async fn test_sol_transfer_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let sol_transfer_instruction = transfer(&fee_payer_pubkey, &recipient_pubkey, 1_000_000);

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(sol_transfer_instruction)
        .build()
        .await
        .expect("Failed to create transaction with SOL transfer");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for SOL transfer policy violation"),
    }
}

#[tokio::test]
async fn test_spl_transfer_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let fee_payer_token_account =
        get_associated_token_address(&fee_payer_pubkey, &setup.usdc_mint.pubkey());
    let recipient_token_account =
        get_associated_token_address(&recipient_pubkey, &setup.usdc_mint.pubkey());

    setup
        .mint_tokens_to_account(&fee_payer_token_account, 100_000)
        .await
        .expect("Failed to mint tokens");

    let spl_transfer_instruction = token_instruction::transfer(
        &spl_token::id(),
        &fee_payer_token_account,
        &recipient_token_account,
        &fee_payer_pubkey,
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create SPL transfer instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(spl_transfer_instruction)
        .build()
        .await
        .expect("Failed to create transaction with SPL transfer");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for SPL transfer policy violation"),
    }
}

#[tokio::test]
async fn test_token2022_transfer_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let fee_payer_token_2022_account = get_associated_token_address_with_program_id(
        &fee_payer_pubkey,
        &setup.usdc_mint_2022.pubkey(),
        &spl_token_2022::id(),
    );
    let recipient_token_2022_account = get_associated_token_address_with_program_id(
        &recipient_pubkey,
        &setup.usdc_mint_2022.pubkey(),
        &spl_token_2022::id(),
    );

    setup
        .mint_tokens_2022_to_account(&fee_payer_token_2022_account, 100_000)
        .await
        .expect("Failed to mint tokens");

    let token_2022_transfer_instruction = token_2022_instruction::transfer_checked(
        &spl_token_2022::id(),
        &fee_payer_token_2022_account,
        &setup.usdc_mint_2022.pubkey(),
        &recipient_token_2022_account,
        &fee_payer_pubkey,
        &[&fee_payer_pubkey],
        1_000,
        USDCMintTestHelper::get_test_usdc_mint_decimals(),
    )
    .expect("Failed to create Token2022 transfer instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(token_2022_transfer_instruction)
        .build()
        .await
        .expect("Failed to create transaction with Token2022 transfer");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for Token2022 transfer policy violation"),
    }
}

#[tokio::test]
async fn test_burn_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account =
        get_associated_token_address(&fee_payer_pubkey, &setup.usdc_mint.pubkey());

    setup
        .mint_tokens_to_account(&fee_payer_token_account, 1_000_000)
        .await
        .expect("Failed to mint SPL");

    let burn_instruction = token_instruction::burn(
        &spl_token::id(),
        &fee_payer_token_account,
        &setup.usdc_mint.pubkey(),
        &fee_payer_pubkey,
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create burn instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(burn_instruction)
        .build()
        .await
        .expect("Failed to create transaction with burn");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for burn policy violation"),
    }
}

#[tokio::test]
async fn test_close_account_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    // Create a new token account to now affect other tests
    let closable_token_account_keypair = Keypair::new();

    let rent = setup
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await
        .expect("Failed to get rent exemption");

    let create_account_ix = create_account(
        &setup.fee_payer_keypair.pubkey(),
        &closable_token_account_keypair.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );

    let create_closable_token_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &closable_token_account_keypair.pubkey(),
        &setup.usdc_mint.pubkey(),
        &setup.fee_payer_keypair.pubkey(),
    )
    .expect("Failed to create initialize account instruction");

    let recent_blockhash = setup.rpc_client.get_latest_blockhash().await.unwrap();
    let setup_tx = Transaction::new_signed_with_payer(
        &[create_account_ix, create_closable_token_account_ix],
        Some(&setup.fee_payer_keypair.pubkey()),
        &[&setup.fee_payer_keypair, &closable_token_account_keypair],
        recent_blockhash,
    );

    setup
        .rpc_client
        .send_and_confirm_transaction(&setup_tx)
        .await
        .expect("Failed to setup and freeze token account");

    let close_account_instruction = token_instruction::close_account(
        &spl_token::id(),
        &closable_token_account_keypair.pubkey(),
        &setup.recipient_pubkey,
        &setup.fee_payer_keypair.pubkey(),
        &[&setup.fee_payer_keypair.pubkey()],
    )
    .expect("Failed to create close account instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(setup.fee_payer_keypair.pubkey())
        .with_instruction(close_account_instruction)
        .build()
        .await
        .expect("Failed to create transaction with close account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for close account policy violation"),
    }
}

#[tokio::test]
async fn test_approve_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer_token_account =
        get_associated_token_address(&fee_payer_pubkey, &setup.usdc_mint.pubkey());

    setup
        .mint_tokens_to_account(&fee_payer_token_account, 1_000_000)
        .await
        .expect("Failed to mint tokens");

    let approve_instruction = token_instruction::approve(
        &spl_token::id(),
        &fee_payer_token_account,
        &recipient_pubkey,
        &fee_payer_pubkey,
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create approve instruction");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_instruction(approve_instruction)
        .build()
        .await
        .expect("Failed to create transaction with approve");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used as source account");
        }
        Ok(_) => panic!("Expected error for approve policy violation"),
    }
}
