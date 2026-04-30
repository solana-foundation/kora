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

#[tokio::test]
async fn test_burn_multisig_bypass() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;
    setup.setup_fee_payer_policy_token_accounts().await.expect("Failed to setup token accounts");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let other_keypair = Keypair::new();
    let multisig_account = Keypair::new();

    // fee payer signs as multisig co-signer, not direct owner
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Multisig::LEN)
        .await
        .expect("Failed to get rent");

    let create_multisig_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &multisig_account.pubkey(),
        rent,
        spl_token_interface::state::Multisig::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_multisig_ix = token_instruction::initialize_multisig(
        &spl_token_interface::id(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey, &other_keypair.pubkey()],
        1,
    )
    .expect("Failed to create init multisig ix");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_multisig_ix, init_multisig_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &multisig_account],
        recent_blockhash,
    );
    ctx.rpc_client().send_and_confirm_transaction(&tx).await.expect("Failed to setup multisig");

    let token_account = Keypair::new();
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await
        .expect("Failed to get rent");

    let create_token_account_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &token_account.pubkey(),
        rent,
        spl_token_interface::state::Account::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_token_account_ix = token_instruction::initialize_account(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &multisig_account.pubkey(),
    )
    .expect("Failed to create init token account ix");

    let tx = Transaction::new_signed_with_payer(
        &[create_token_account_ix, init_token_account_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &token_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup token account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint");

    let burn_ix = token_instruction::burn(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create burn ix");

    let bypass_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_payment(
            &setup.fee_payer_policy_mint.pubkey(),
            &setup.sender_keypair.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .with_instruction(burn_ix)
        .build()
        .await
        .expect("Failed to build tx");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![bypass_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Burn'");
        }
        Ok(_) => panic!("Expected error for burn multisig bypass policy violation"),
    }
}

#[tokio::test]
async fn test_approve_multisig_bypass() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;
    setup.setup_fee_payer_policy_token_accounts().await.expect("Failed to setup token accounts");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let other_keypair = Keypair::new();
    let multisig_account = Keypair::new();

    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Multisig::LEN)
        .await
        .expect("Failed to get rent");

    let create_multisig_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &multisig_account.pubkey(),
        rent,
        spl_token_interface::state::Multisig::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_multisig_ix = token_instruction::initialize_multisig(
        &spl_token_interface::id(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey, &other_keypair.pubkey()],
        1,
    )
    .expect("Failed to create init multisig ix");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_multisig_ix, init_multisig_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &multisig_account],
        recent_blockhash,
    );
    ctx.rpc_client().send_and_confirm_transaction(&tx).await.expect("Failed to setup multisig");

    let token_account = Keypair::new();
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await
        .expect("Failed to get rent");

    let create_token_account_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &token_account.pubkey(),
        rent,
        spl_token_interface::state::Account::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_token_account_ix = token_instruction::initialize_account(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &multisig_account.pubkey(),
    )
    .expect("Failed to create init token account ix");

    let tx = Transaction::new_signed_with_payer(
        &[create_token_account_ix, init_token_account_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &token_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup token account");

    let recipient_keypair = Keypair::new();
    let approve_ix = token_instruction::approve(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        &recipient_keypair.pubkey(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create approve ix");

    let bypass_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_payment(
            &setup.fee_payer_policy_mint.pubkey(),
            &setup.sender_keypair.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .with_instruction(approve_ix)
        .build()
        .await
        .expect("Failed to build tx");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![bypass_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Approve'");
        }
        Ok(_) => panic!("Expected error for approve multisig bypass policy violation"),
    }
}

#[tokio::test]
async fn test_transfer_multisig_bypass() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;
    setup.setup_fee_payer_policy_token_accounts().await.expect("Failed to setup token accounts");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let other_keypair = Keypair::new();
    let multisig_account = Keypair::new();

    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Multisig::LEN)
        .await
        .expect("Failed to get rent");

    let create_multisig_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &multisig_account.pubkey(),
        rent,
        spl_token_interface::state::Multisig::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_multisig_ix = token_instruction::initialize_multisig(
        &spl_token_interface::id(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey, &other_keypair.pubkey()],
        1,
    )
    .expect("Failed to create init multisig ix");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_multisig_ix, init_multisig_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &multisig_account],
        recent_blockhash,
    );
    ctx.rpc_client().send_and_confirm_transaction(&tx).await.expect("Failed to setup multisig");

    let source_token_account = Keypair::new();
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await
        .expect("Failed to get rent");

    let create_source_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &source_token_account.pubkey(),
        rent,
        spl_token_interface::state::Account::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_source_ix = token_instruction::initialize_account(
        &spl_token_interface::id(),
        &source_token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &multisig_account.pubkey(),
    )
    .expect("Failed to create init source account ix");

    let tx = Transaction::new_signed_with_payer(
        &[create_source_ix, init_source_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &source_token_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup source account");

    setup
        .mint_fee_payer_policy_tokens_to_account(&source_token_account.pubkey(), 1_000_000)
        .await
        .expect("Failed to mint");

    let recipient_keypair = Keypair::new();
    let recipient_token_account = get_associated_token_address(
        &recipient_keypair.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
    );
    let create_recipient_ata_ix =
        spl_associated_token_account_interface::instruction::create_associated_token_account(
            &setup.sender_keypair.pubkey(),
            &recipient_keypair.pubkey(),
            &setup.fee_payer_policy_mint.pubkey(),
            &spl_token_interface::id(),
        );

    let tx = Transaction::new_signed_with_payer(
        &[create_recipient_ata_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup recipient account");

    let transfer_ix = token_instruction::transfer(
        &spl_token_interface::id(),
        &source_token_account.pubkey(),
        &recipient_token_account,
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey],
        1_000,
    )
    .expect("Failed to create transfer ix");

    let bypass_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_payment(
            &setup.fee_payer_policy_mint.pubkey(),
            &setup.sender_keypair.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .with_instruction(transfer_ix)
        .build()
        .await
        .expect("Failed to build tx");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![bypass_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token Transfer'");
        }
        Ok(_) => panic!("Expected error for transfer multisig bypass policy violation"),
    }
}

#[tokio::test]
async fn test_set_authority_multisig_bypass() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;
    setup.setup_fee_payer_policy_token_accounts().await.expect("Failed to setup token accounts");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let other_keypair = Keypair::new();
    let multisig_account = Keypair::new();

    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Multisig::LEN)
        .await
        .expect("Failed to get rent");

    let create_multisig_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &multisig_account.pubkey(),
        rent,
        spl_token_interface::state::Multisig::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_multisig_ix = token_instruction::initialize_multisig(
        &spl_token_interface::id(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey, &other_keypair.pubkey()],
        1,
    )
    .expect("Failed to create init multisig ix");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_multisig_ix, init_multisig_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &multisig_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup multisig account");

    let token_account = Keypair::new();
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await
        .expect("Failed to get rent");

    let create_account_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &token_account.pubkey(),
        rent,
        spl_token_interface::state::Account::LEN as u64,
        &spl_token_interface::id(),
    );

    let init_account_ix = token_instruction::initialize_account(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        &setup.fee_payer_policy_mint.pubkey(),
        &multisig_account.pubkey(),
    )
    .expect("Failed to create init account ix");

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_account_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &token_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup token account");

    // fee payer signs as multisig co-signer, not direct owner
    let set_authority_ix = token_instruction::set_authority(
        &spl_token_interface::id(),
        &token_account.pubkey(),
        Some(&other_keypair.pubkey()),
        token_instruction::AuthorityType::AccountOwner,
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey],
    )
    .expect("Failed to create set_authority ix");

    let bypass_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_payment(
            &setup.fee_payer_policy_mint.pubkey(),
            &setup.sender_keypair.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .with_instruction(set_authority_ix)
        .build()
        .await
        .expect("Failed to build tx");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![bypass_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message("Fee payer cannot be used for 'SPL Token SetAuthority'");
        }
        Ok(_) => panic!("Expected error for set_authority multisig bypass policy violation"),
    }
}

#[tokio::test]
async fn test_reallocate_multisig_bypass() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let setup = TestAccountSetup::new().await;
    setup.setup_fee_payer_policy_token_accounts().await.expect("Failed to setup token accounts");

    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let other_keypair = Keypair::new();
    let multisig_account = Keypair::new();

    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Multisig::LEN)
        .await
        .expect("Failed to get rent");

    let create_multisig_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &multisig_account.pubkey(),
        rent,
        spl_token_interface::state::Multisig::LEN as u64,
        &spl_token_2022_interface::id(),
    );

    let init_multisig_ix = token_2022_instruction::initialize_multisig(
        &spl_token_2022_interface::id(),
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey, &other_keypair.pubkey()],
        1,
    )
    .expect("Failed to create init multisig ix");

    let recent_blockhash =
        ctx.rpc_client().get_latest_blockhash().await.expect("Failed to get blockhash");
    let tx = Transaction::new_signed_with_payer(
        &[create_multisig_ix, init_multisig_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &multisig_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup multisig account");

    let token_account = Keypair::new();
    let rent = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await
        .expect("Failed to get rent");

    let create_account_ix = create_account(
        &setup.sender_keypair.pubkey(),
        &token_account.pubkey(),
        rent,
        spl_token_interface::state::Account::LEN as u64,
        &spl_token_2022_interface::id(),
    );

    let init_account_ix = token_2022_instruction::initialize_account(
        &spl_token_2022_interface::id(),
        &token_account.pubkey(),
        &setup.fee_payer_policy_mint_2022.pubkey(),
        &multisig_account.pubkey(),
    )
    .expect("Failed to create init account ix");

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_account_ix],
        Some(&setup.sender_keypair.pubkey()),
        &[&setup.sender_keypair, &token_account],
        recent_blockhash,
    );
    ctx.rpc_client()
        .send_and_confirm_transaction(&tx)
        .await
        .expect("Failed to setup token account");

    // fee payer signs as multisig co-signer, not direct owner
    let reallocate_ix = token_2022_instruction::reallocate(
        &spl_token_2022_interface::id(),
        &token_account.pubkey(),
        &fee_payer_pubkey,
        &multisig_account.pubkey(),
        &[&fee_payer_pubkey],
        &[],
    )
    .expect("Failed to create reallocate ix");

    let bypass_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer_pubkey)
        .with_spl_payment(
            &setup.fee_payer_policy_mint.pubkey(),
            &setup.sender_keypair.pubkey(),
            &fee_payer_pubkey,
            1_000_000,
        )
        .with_instruction(reallocate_ix)
        .build()
        .await
        .expect("Failed to build tx");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![bypass_tx]).await;

    match result {
        Err(error) => {
            error.assert_contains_message(
                "Token2022 Reallocate is not allowed when involving fee payer",
            );
        }
        Ok(_) => panic!("Expected error for reallocate multisig bypass policy violation"),
    }
}
