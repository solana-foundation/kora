use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signature::{Keypair, Signer};

/// Test transaction limit enforcement for bundles - each tx in bundle counts toward limit
/// Config: windowed=4/30s, lifetime=5
/// Expects: Bundle with 4 txs succeeds, then single tx fails (windowed limit reached)
#[tokio::test]
async fn test_bundle_transaction_limit_enforcement() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Create a bundle with 4 transactions (hits windowed limit exactly)
    let mut transactions = Vec::new();
    for _ in 0..4 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build transaction");
        transactions.push(tx_b64);
    }

    // Bundle with 4 transactions should succeed (windowed limit is 4)
    let response: serde_json::Value = ctx
        .rpc_call("signBundle", rpc_params![transactions, None::<String>, false, user_id.clone()])
        .await
        .expect("Failed to sign bundle with 4 transactions");

    response.assert_success();
    assert!(
        response["signed_transactions"].as_array().is_some(),
        "Expected signed_transactions array in response"
    );

    // 5th transaction (single) should fail (exceeds windowed limit of 4)
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transaction");

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signBundle",
            rpc_params![vec![tx_b64], None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for exceeding windowed limit after bundle");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test instruction limit enforcement for bundles - instructions across all txs count
/// Config: max 3 CreateAccount instructions per wallet (lifetime)
/// Expects: Bundle with 3 CreateAccount txs succeeds, 4th fails
#[tokio::test]
async fn test_bundle_instruction_limit_enforcement() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();

    // Create a bundle with 3 CreateAccount instructions (hits instruction limit exactly)
    let mut transactions = Vec::new();
    let mut new_accounts = Vec::new();

    for _ in 0..3 {
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
        transactions.push(tx_b64);
        new_accounts.push(new_account);
    }

    // Bundle with 3 CreateAccount instructions should succeed (limit is 3)
    let response: serde_json::Value = ctx
        .rpc_call("signBundle", rpc_params![transactions, None::<String>, false, user_id.clone()])
        .await
        .expect("Failed to sign bundle with 3 CreateAccount transactions");

    response.assert_success();

    // 4th CreateAccount should fail (exceeds instruction limit)
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
            "signBundle",
            rpc_params![vec![tx_b64], None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for 4th CreateAccount exceeding instruction limit");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test mixed rules for bundles - transaction and instruction limits both apply
/// Config: max 3 CreateAccount (lifetime), max 4 tx/30s (windowed), max 5 tx (lifetime)
/// Tests bundle with mixed instruction types
#[tokio::test]
async fn test_bundle_mixed_rules_enforcement() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Create a bundle with 2 CreateAccount + 1 transfer (3 txs total)
    let mut transactions = Vec::new();
    let mut new_accounts = Vec::new();

    // 2 CreateAccount instructions
    for _ in 0..2 {
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
            .expect("Failed to build CreateAccount transaction");
        transactions.push(tx_b64);
        new_accounts.push(new_account);
    }

    // 1 transfer (no CreateAccount)
    let tx_b64 = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(&sender.pubkey(), &recipient, 1000)
        .with_signer(&sender)
        .build()
        .await
        .expect("Failed to build transfer transaction");
    transactions.push(tx_b64);

    // First bundle should succeed (2 CreateAccount of 3 limit, 3 tx of 4 windowed limit)
    let response: serde_json::Value = ctx
        .rpc_call("signBundle", rpc_params![transactions, None::<String>, false, user_id.clone()])
        .await
        .expect("Failed to sign mixed bundle");

    response.assert_success();

    // Second bundle with 2 CreateAccount should fail (would be 4 total, exceeds limit of 3)
    let mut transactions2 = Vec::new();
    for _ in 0..2 {
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
            .expect("Failed to build CreateAccount transaction");
        transactions2.push(tx_b64);
    }

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signBundle",
            rpc_params![transactions2, None::<String>, false, user_id.clone()],
        )
        .await;

    let err =
        result.expect_err("Expected error for exceeding instruction limit with second bundle");
    err.assert_contains_message("Usage limit exceeded");
}

/// Test bundle fails fast on first transaction exceeding limit
/// Config: windowed=4/30s
/// Bundle with 5 transactions should fail immediately (not process any)
#[tokio::test]
async fn test_bundle_fails_fast_on_limit_exceeded() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let sender = create_funded_wallet(&ctx).await;
    let user_id = sender.pubkey().to_string();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    // Create a bundle with 5 transactions (exceeds windowed limit of 4)
    let mut transactions = Vec::new();
    for _ in 0..5 {
        let tx_b64 = ctx
            .transaction_builder()
            .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
            .with_transfer(&sender.pubkey(), &recipient, 1000)
            .with_signer(&sender)
            .build()
            .await
            .expect("Failed to build transaction");
        transactions.push(tx_b64);
    }

    // Bundle with 5 transactions should fail (exceeds windowed limit of 4)
    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signBundle",
            rpc_params![transactions, None::<String>, false, user_id.clone()],
        )
        .await;

    let err = result.expect_err("Expected error for bundle exceeding windowed limit");
    err.assert_contains_message("Usage limit exceeded");
}
