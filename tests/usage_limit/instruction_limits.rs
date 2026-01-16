use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::{Keypair, Signer};

/// Test System CreateAccount instruction limit - count only CreateAccount instructions
/// Config: max 3 CreateAccount instructions per wallet (lifetime)
#[tokio::test]
async fn test_system_create_account_instruction_limit() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();

    // First 3 CreateAccount transactions should succeed (instruction limit is 3)
    for i in 1..=3 {
        let new_account = Keypair::new();
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_system_create_account(
                &FeePayerTestHelper::get_fee_payer_pubkey(),
                &new_account.pubkey(),
                1000000,
                0,
                &sender.pubkey(),
            )
            .with_signer(&sender)
            .with_signer(&new_account)
            .build()
            .await
            .expect("Failed to build transaction with CreateAccount");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to sign CreateAccount transaction #{i}"));

        response.assert_success();
    }

    // 4th CreateAccount should fail with usage limit exceeded error
    let new_account = Keypair::new();
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_signer(&sender)
        .with_signer(&new_account)
        .build()
        .await
        .expect("Failed to build transaction with CreateAccount");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for 4th CreateAccount exceeding instruction limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test multiple CreateAccount instructions in one transaction - count all matching instructions
/// Config: max 3 CreateAccount instructions per wallet (lifetime)
#[tokio::test]
async fn test_multiple_create_account_instructions_in_one_tx() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let new_account1 = Keypair::new();
    let new_account2 = Keypair::new();

    // Create transaction with 2 System CreateAccount instructions
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account1.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account2.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_signer(&sender)
        .with_signer(&new_account1)
        .with_signer(&new_account2)
        .build()
        .await
        .expect("Failed to build transaction with multiple CreateAccounts");

    // First transaction with 2 CreateAccount instructions should succeed (counts as 2, limit is 3)
    let response: serde_json::Value = ctx
        .rpc_call(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await
        .expect("Failed to sign transaction with multiple CreateAccounts");

    response.assert_success();

    // Second transaction with 2 CreateAccount instructions should fail (2 + 2 = 4, exceeds limit of 3)
    let new_account3 = Keypair::new();
    let new_account4 = Keypair::new();

    let tx_b64_2 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account3.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account4.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_signer(&sender)
        .with_signer(&new_account3)
        .with_signer(&new_account4)
        .build()
        .await
        .expect("Failed to build second transaction with multiple CreateAccounts");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64_2, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result
        .expect_err("Expected error for exceeding instruction limit with multiple instructions");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test non-matching instructions - don't count unrelated instructions
/// Config: max 3 CreateAccount instructions (lifetime), max 4 transactions per 30s (windowed)
#[tokio::test]
async fn test_non_matching_instructions_not_counted() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Send 4 SOL transfers (windowed limit) - these should NOT count toward CreateAccount limit
    for i in 1..=4 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build SOL transfer transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to sign SOL transfer transaction #{i}"));

        response.assert_success();
    }

    // 5th transaction should fail due to windowed transaction limit (4 per 30s), not instruction limit
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build SOL transfer transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for exceeding transaction windowed limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test instruction limit independent from transaction limit
/// Config: max 3 CreateAccount instructions (lifetime), max 5 transactions (lifetime)
#[tokio::test]
async fn test_instruction_limit_independent_from_transaction_limit() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();

    // Use up instruction limit (3 CreateAccounts)
    for _ in 1..=3 {
        let new_account = Keypair::new();
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_system_create_account(
                &FeePayerTestHelper::get_fee_payer_pubkey(),
                &new_account.pubkey(),
                1000000,
                0,
                &sender.pubkey(),
            )
            .with_signer(&sender)
            .with_signer(&new_account)
            .build()
            .await
            .expect("Failed to build transaction");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .expect("Failed to sign transaction");

        response.assert_success();
    }

    // 4th CreateAccount should fail due to instruction limit (even though transaction limit allows more)
    let new_account = Keypair::new();
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_system_create_account(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &new_account.pubkey(),
            1000000,
            0,
            &sender.pubkey(),
        )
        .with_signer(&sender)
        .with_signer(&new_account)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for exceeding instruction limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test ATA CreateIdempotent instruction limit - count only CreateIdempotent instructions
/// Config: max 3 CreateIdempotent instructions per wallet (lifetime)
/// This test creates ATAs along with a SOL transfer to ensure the sender wallet is tracked
#[tokio::test]
async fn test_ata_create_idempotent_instruction_limit() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // First 3 CreateIdempotent transactions should succeed (instruction limit is 3)
    for i in 1..=3 {
        // Each iteration creates ATA for a different owner (new keypair)
        let owner = Keypair::new();
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            // Include a transfer so sender signs and usage is tracked against sender wallet
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_create_ata_idempotent(&usdc_mint, &owner.pubkey())
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build transaction with CreateIdempotent");

        let response: serde_json::Value = ctx
            .rpc_call(
                "signAndSendTransaction",
                rpc_params![tx_b64, None::<String>, false, user_id.clone()],
            )
            .await
            .unwrap_or_else(|e| panic!("Failed to sign CreateIdempotent transaction #{i}: {e}"));

        response.assert_success();
    }

    // 4th CreateIdempotent should fail with usage limit exceeded error
    let owner = Keypair::new();
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_create_ata_idempotent(&usdc_mint, &owner.pubkey())
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction with CreateIdempotent");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signAndSendTransaction",
            rpc_params![tx_b64, None::<String>, false, user_id.clone()],
        )
        .await;

    let err =
        result.expect_err("Expected error for 4th CreateIdempotent exceeding instruction limit");
    err.assert_contains_message("Usage limit exceeded");
}
