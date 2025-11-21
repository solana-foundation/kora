use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use solana_system_interface::instruction::{create_account, transfer};
use spl_associated_token_account_interface::address::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token_2022_interface::instruction as token_2022_instruction;
use spl_token_interface::instruction as token_instruction;

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
            error.assert_contains_message("Fee payer cannot be used for 'System Transfer'");
        }
        Ok(_) => panic!("Expected error for SOL transfer policy violation"),
    }
}

#[tokio::test]
async fn test_assign_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let new_owner = Pubkey::new_unique();

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_system_assign(&fee_payer_pubkey, &new_owner)
        .build()
        .await
        .expect("Failed to create transaction with assign");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'System Assign'");
        }
        Ok(_) => panic!("Expected error for assign policy violation"),
    }
}

#[tokio::test]
async fn test_create_account_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let new_account = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_system_create_account(&fee_payer_pubkey, &new_account, 1_000_000, 0, &owner)
        .build()
        .await
        .expect("Failed to create transaction with create_account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'System Create Account'");
        }
        Ok(_) => panic!("Expected error for create_account policy violation"),
    }
}

#[tokio::test]
async fn test_allocate_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_system_allocate(&fee_payer_pubkey, 1024)
        .build()
        .await
        .expect("Failed to create transaction with allocate");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'System Allocate'");
        }
        Ok(_) => panic!("Expected error for allocate policy violation"),
    }
}

#[tokio::test]
async fn test_spl_transfer_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");
    let recipient_token_account =
        get_associated_token_address(&recipient_pubkey, &setup.fee_payer_policy_mint.pubkey());

    setup
        .mint_fee_payer_policy_tokens_to_account(&fee_payer_token_account.pubkey(), 100_000)
        .await
        .expect("Failed to mint tokens");

    let spl_transfer_instruction = token_instruction::transfer(
        &spl_token_interface::id(),
        &fee_payer_token_account.pubkey(),
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
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Transfer'");
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

    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");
    let recipient_token_2022_account = get_associated_token_address_with_program_id(
        &recipient_pubkey,
        &setup.fee_payer_policy_mint_2022.pubkey(),
        &spl_token_2022_interface::id(),
    );

    setup
        .mint_fee_payer_policy_tokens_2022_to_account(
            &fee_payer_token_2022_account.pubkey(),
            100_000,
        )
        .await
        .expect("Failed to mint tokens");

    let token_2022_transfer_instruction = token_2022_instruction::transfer_checked(
        &spl_token_2022_interface::id(),
        &fee_payer_token_2022_account.pubkey(),
        &setup.fee_payer_policy_mint_2022.pubkey(),
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
            error
                .assert_contains_message("Fee payer cannot be used for 'Token2022 Token Transfer'");
        }
        Ok(_) => panic!("Expected error for Token2022 transfer policy violation"),
    }
}

#[tokio::test]
async fn test_burn_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&fee_payer_token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint SPL");

    let burn_instruction = token_instruction::burn(
        &spl_token_interface::id(),
        &fee_payer_token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
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
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Burn'");
        }
        Ok(_) => panic!("Expected error for burn policy violation"),
    }
}

#[tokio::test]
async fn test_close_account_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    let close_account_instruction = token_instruction::close_account(
        &spl_token_interface::id(),
        &fee_payer_token_account.pubkey(),
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
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Close Account'");
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
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&fee_payer_token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint tokens");

    let approve_instruction = token_instruction::approve(
        &spl_token_interface::id(),
        &fee_payer_token_account.pubkey(),
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
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Approve'");
        }
        Ok(_) => panic!("Expected error for approve policy violation"),
    }
}

#[tokio::test]
async fn test_revoke_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&fee_payer_token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint tokens");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_revoke(&fee_payer_token_account.pubkey(), &fee_payer_pubkey)
        .build()
        .await
        .expect("Failed to create transaction with revoke");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Revoke'");
        }
        Ok(_) => panic!("Expected error for revoke policy violation"),
    }
}

#[tokio::test]
async fn test_revoke_token2022_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_2022_to_account(
            &fee_payer_token_2022_account.pubkey(),
            1_000_000,
        )
        .await
        .expect("Failed to mint Token2022");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_token2022_revoke(&fee_payer_token_2022_account.pubkey(), &fee_payer_pubkey)
        .build()
        .await
        .expect("Failed to create transaction with Token2022 revoke");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'Token2022 Token Revoke'");
        }
        Ok(_) => panic!("Expected error for Token2022 revoke policy violation"),
    }
}

#[tokio::test]
async fn test_set_authority_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&fee_payer_token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint tokens");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_set_authority(
            &fee_payer_token_account.pubkey(),
            Some(&recipient_pubkey),
            token_instruction::AuthorityType::AccountOwner,
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with set_authority");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token SetAuthority'");
        }
        Ok(_) => panic!("Expected error for set_authority policy violation"),
    }
}

#[tokio::test]
async fn test_set_authority_token2022_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");

    setup
        .mint_fee_payer_policy_tokens_2022_to_account(
            &fee_payer_token_2022_account.pubkey(),
            1_000_000,
        )
        .await
        .expect("Failed to mint Token2022");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_token2022_set_authority(
            &fee_payer_token_2022_account.pubkey(),
            Some(&recipient_pubkey),
            // Can't use freeze authority on token2022 account, so use close authority
            token_2022_instruction::AuthorityType::CloseAccount,
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with Token2022 set_authority");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message(
                "Fee payer cannot be used for 'Token2022 Token SetAuthority'",
            );
        }
        Ok(_) => panic!("Expected error for Token2022 set_authority policy violation"),
    }
}

#[tokio::test]
async fn test_mint_to_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_mint_to(
            &setup.fee_payer_policy_mint.pubkey(),
            &fee_payer_token_account.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .build()
        .await
        .expect("Failed to create transaction with mint_to");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token MintTo'");
        }
        Ok(_) => panic!("Expected error for mint_to policy violation"),
    }
}

#[tokio::test]
async fn test_mint_to_token2022_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_token2022_mint_to(
            &setup.fee_payer_policy_mint_2022.pubkey(),
            &fee_payer_token_2022_account.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .build()
        .await
        .expect("Failed to create transaction with Token2022 mint_to");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'Token2022 Token MintTo'");
        }
        Ok(_) => panic!("Expected error for Token2022 mint_to policy violation"),
    }
}

#[tokio::test]
async fn test_freeze_account_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_freeze_account(
            &fee_payer_token_account.pubkey(),
            &setup.fee_payer_policy_mint.pubkey(),
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with freeze_account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token FreezeAccount'");
        }
        Ok(_) => panic!("Expected error for freeze_account policy violation"),
    }
}

#[tokio::test]
async fn test_freeze_account_token2022_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_token2022_freeze_account(
            &fee_payer_token_2022_account.pubkey(),
            &setup.fee_payer_policy_mint_2022.pubkey(),
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with Token2022 freeze_account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message(
                "Fee payer cannot be used for 'Token2022 Token FreezeAccount'",
            );
        }
        Ok(_) => panic!("Expected error for Token2022 freeze_account policy violation"),
    }
}

#[tokio::test]
async fn test_thaw_account_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_account = setup
        .create_fee_payer_token_account_spl(&setup.fee_payer_policy_mint.pubkey())
        .await
        .expect("Failed to create token account");

    // Freeze the account first (directly on-chain, bypassing Kora validator)
    let freeze_ix = spl_token_interface::instruction::freeze_account(
        &spl_token_interface::id(),
        &fee_payer_token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &fee_payer_pubkey,
        &[],
    )
    .expect("Failed to create freeze instruction");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let freeze_tx = Transaction::new_signed_with_payer(
        &[freeze_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &setup.fee_payer_keypair],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&freeze_tx)
        .await
        .expect("Failed to freeze account");

    // Now thaw - fee_payer has authority but policy should reject
    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_thaw_account(
            &fee_payer_token_account.pubkey(),
            &setup.fee_payer_policy_mint.pubkey(),
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with thaw_account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token ThawAccount'");
        }
        Ok(_) => panic!("Expected error for thaw_account policy violation"),
    }
}

#[tokio::test]
async fn test_thaw_account_token2022_policy_violation() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let fee_payer_token_2022_account = setup
        .create_fee_payer_token_account_2022(&setup.fee_payer_policy_mint_2022.pubkey())
        .await
        .expect("Failed to create token account");

    // Freeze the account first (directly on-chain, bypassing Kora validator)
    let freeze_ix = spl_token_2022_interface::instruction::freeze_account(
        &spl_token_2022_interface::id(),
        &fee_payer_token_2022_account.pubkey(),
        &setup.fee_payer_policy_mint_2022.pubkey(),
        &fee_payer_pubkey,
        &[],
    )
    .expect("Failed to create freeze instruction");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let freeze_tx = Transaction::new_signed_with_payer(
        &[freeze_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &setup.fee_payer_keypair],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&freeze_tx)
        .await
        .expect("Failed to freeze account");

    // Now thaw - fee_payer has authority but policy should reject
    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_token2022_thaw_account(
            &fee_payer_token_2022_account.pubkey(),
            &setup.fee_payer_policy_mint_2022.pubkey(),
            &fee_payer_pubkey,
        )
        .build()
        .await
        .expect("Failed to create transaction with Token2022 thaw_account");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message(
                "Fee payer cannot be used for 'Token2022 Token ThawAccount'",
            );
        }
        Ok(_) => panic!("Expected error for Token2022 thaw_account policy violation"),
    }
}
