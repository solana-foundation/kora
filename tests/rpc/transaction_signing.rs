use std::str::FromStr;

use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::transaction::TransactionUtil;
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use tests::common::helpers::get_fee_for_default_transaction_in_usdc;

// **************************************************************************************
// Sign transaction tests
// **************************************************************************************

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

/// Test sign V0 transaction with valid lookup table
#[tokio::test]
async fn test_sign_transaction_v0_with_valid_lookup_table() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    let allowed_lookup_table_address =
        LookupTableHelper::get_allowed_lookup_table_address().unwrap();

    // Create a V0 transaction using the allowed lookup table (index 0)
    let v0_transaction = ctx
        .v0_transaction_builder_with_lookup(vec![allowed_lookup_table_address])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction with allowed lookup table");

    // Test signing the V0 transaction through Kora RPC - this should succeed
    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![v0_transaction])
        .await
        .expect("Failed to sign V0 transaction with valid lookup table");

    response.assert_success();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction to ensure it's valid
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction simulation failed");
}

/// Test sign V0 transaction with invalid lookup table
#[tokio::test]
async fn test_sign_transaction_v0_with_invalid_lookup_table() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    // Create a V0 transaction using the disallowed lookup table (index 1)
    let disallowed_lookup_table_address =
        LookupTableHelper::get_disallowed_lookup_table_address().unwrap();

    let v0_transaction = ctx
        .v0_transaction_builder_with_lookup(vec![disallowed_lookup_table_address])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &LookupTableHelper::get_test_disallowed_address()
                .expect("Failed to get test disallowed address"),
            10,
        )
        .build()
        .await
        .expect("Failed to create V0 transaction with disallowed lookup table");

    // Test signing the V0 transaction through Kora RPC - this should fail due to disallowed addresses
    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![v0_transaction]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

#[tokio::test]
async fn test_sign_transaction_invalid_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let invalid_tx = "invalid_base64_transaction";

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![invalid_tx]).await;

    assert!(result.is_err(), "Expected error for invalid transaction");
}

// **************************************************************************************
// Sign and send transaction tests
// **************************************************************************************

#[tokio::test]
async fn test_sign_and_send_transaction_legacy() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_transfer(&sender.pubkey(), &recipient, 10)
        .build()
        .await
        .expect("Failed to create signed test transaction");

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed");
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_and_send_transaction_v0() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let test_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
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

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed");
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_and_send_transaction_v0_with_lookup() {
    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let ctx = TestContext::new().await.expect("Failed to create test context");

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    let test_tx = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
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

    let result: Result<serde_json::Value, _> =
        ctx.rpc_call("signAndSendTransaction", rpc_params![test_tx]).await;

    assert!(result.is_ok(), "Expected signAndSendTransaction to succeed");
    let response = result.unwrap();

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

// **************************************************************************************
// Sign transaction if paid tests
// **************************************************************************************

/// Test complex sign transaction if paid with fee payer pool logic
#[tokio::test]
async fn test_sign_transaction_if_paid_legacy() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();

    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Use transaction builder with proper signing and automatic ATA derivation
    let base64_transaction = ctx
        .transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            get_fee_for_default_transaction_in_usdc(),
        )
        .with_spl_transfer(&token_mint, &sender.pubkey(), &recipient, 1)
        .build()
        .await
        .expect("Failed to create signed transaction");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign transaction");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base64 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate transaction");

    assert!(simulated_tx.value.err.is_none(), "Transaction simulation failed");
}

/// Test sign transaction if paid with V0 transaction
#[tokio::test]
async fn test_sign_transaction_if_paid_v0() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let base64_transaction = ctx
        .v0_transaction_builder()
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            1,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 signed transaction");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 transaction");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base64 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction simulation failed");
}

/// Test sign transaction if paid with V0 transaction and lookup table
#[tokio::test]
async fn test_sign_transaction_if_paid_v0_with_lookup() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let rpc_client = ctx.rpc_client();

    // Get fee payer from config (use first one from the pool)
    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    let fee_payers = response["fee_payers"].as_array().unwrap();
    let fee_payer = Pubkey::from_str(fee_payers[0].as_str().unwrap()).unwrap();

    let sender = SenderTestHelper::get_test_sender_keypair();
    let recipient = RecipientTestHelper::get_recipient_pubkey();
    let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let transaction_lookup_table = LookupTableHelper::get_transaction_lookup_table_address()
        .expect("Failed to get transaction lookup table from fixtures");

    // Use V0 transaction builder with lookup table and proper signing
    let base64_transaction = ctx
        .v0_transaction_builder_with_lookup(vec![transaction_lookup_table])
        .with_fee_payer(fee_payer)
        .with_signer(&sender)
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &fee_payer,
            get_fee_for_default_transaction_in_usdc(),
            TEST_USDC_MINT_DECIMALS,
        )
        .with_spl_transfer_checked(
            &token_mint,
            &sender.pubkey(),
            &recipient,
            1,
            TEST_USDC_MINT_DECIMALS,
        )
        .build()
        .await
        .expect("Failed to create V0 signed transaction with lookup table");

    // Test signTransactionIfPaid
    let response: serde_json::Value = ctx
        .rpc_call("signTransactionIfPaid", rpc_params![base64_transaction])
        .await
        .expect("Failed to sign V0 transaction with lookup table");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );

    // Decode the base64 transaction string
    let transaction_string = response["signed_transaction"].as_str().unwrap();
    let transaction = TransactionUtil::decode_b64_transaction(transaction_string)
        .expect("Failed to decode transaction from base64");

    // Simulate the transaction
    let simulated_tx = rpc_client
        .simulate_transaction(&transaction)
        .await
        .expect("Failed to simulate V0 transaction with lookup table");

    assert!(simulated_tx.value.err.is_none(), "V0 transaction with lookup table simulation failed");
}
