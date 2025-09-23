use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{
    program_pack::Pack, signature::Keypair, signer::Signer, transaction::Transaction,
};
use solana_system_interface::instruction::create_account;
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;

#[tokio::test]
async fn test_frozen_token_account_as_fee_payment() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let frozen_token_account_keypair = Keypair::new();

    let rent = setup
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .await
        .expect("Failed to get rent exemption");

    let create_account_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &frozen_token_account_keypair.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );

    let create_frozen_token_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &frozen_token_account_keypair.pubkey(),
        &setup.usdc_mint.pubkey(),
        &setup.sender_keypair.pubkey(),
    )
    .expect("Failed to create initialize account instruction");

    let mint_tokens_ix = token_instruction::mint_to(
        &spl_token::id(),
        &setup.usdc_mint.pubkey(),
        &frozen_token_account_keypair.pubkey(),
        &setup.sender_keypair.pubkey(),
        &[&setup.sender_keypair.pubkey()],
        100_000,
    )
    .expect("Failed to create mint instruction");

    let freeze_instruction = token_instruction::freeze_account(
        &spl_token::id(),
        &frozen_token_account_keypair.pubkey(),
        &setup.usdc_mint.pubkey(),
        &setup.sender_keypair.pubkey(),
        &[&setup.sender_keypair.pubkey()],
    )
    .expect("Failed to create freeze instruction");

    let recent_blockhash = setup.rpc_client.get_latest_blockhash().await.unwrap();
    let setup_tx = Transaction::new_signed_with_payer(
        &[create_account_ix, create_frozen_token_account_ix, mint_tokens_ix, freeze_instruction],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &frozen_token_account_keypair],
        recent_blockhash,
    );

    setup
        .rpc_client
        .send_and_confirm_transaction(&setup_tx)
        .await
        .expect("Failed to setup and freeze token account");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_spl_payment_with_accounts(
            &frozen_token_account_keypair.pubkey(),
            &get_associated_token_address(
                &FeePayerTestHelper::get_fee_payer_pubkey(),
                &setup.usdc_mint.pubkey(),
            ),
            &setup.sender_keypair.pubkey(),
            50_000,
        )
        .build()
        .await
        .expect("Failed to create transaction with frozen fee payer token account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        // 0x11: Frozen token account
        Err(error) => {
            error.assert_contains_message("custom program error: 0x11");
        }
        Ok(_) => panic!("Expected error for transaction with frozen fee payment account"),
    }
}
