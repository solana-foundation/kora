use crate::common::*;
use jsonrpsee::rpc_params;
use solana_sdk::signer::Signer;

/// Every test in this module runs against a node whose preferred RPC endpoint
/// refuses connections, so a pass means the request was retried against the
/// fallback rather than served by the primary.
///
/// Every method exercised here makes at least one Solana RPC call. A method that
/// only reads in-process config would pass whether or not failover works, so it
/// has no place in this suite.
///
/// This is the coverage the failover has to have: the requests below are not all
/// one call to the Solana client. `getBlockhash` resolves through the account
/// cache, `estimateTransactionFee` walks transaction validation, mint and token
/// lookups, `signTransaction` resolves lookup tables. Each of those holds nothing
/// but a `&RpcClient`, so if failover only worked for a subset of call sites these
/// tests would catch it.

/// The simplest read path: a blockhash fetch.
#[tokio::test]
async fn test_get_blockhash_fails_over() {
    let ctx = crate::ctx().await;

    let response: serde_json::Value =
        ctx.rpc_call("getBlockhash", rpc_params![]).await.expect("getBlockhash should fail over");

    response.assert_success();
    response.assert_has_field("blockhash");
    response.assert_valid_blockhash();
}

/// Fee estimation reaches account fetches, mint decimals, token balances and the
/// blockhash through several layers that each only receive a `&RpcClient`.
#[tokio::test]
async fn test_estimate_transaction_fee_fails_over() {
    let ctx = crate::ctx().await;

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
        .rpc_call("estimateTransactionFee", rpc_params![test_tx])
        .await
        .expect("estimateTransactionFee should fail over");

    response.assert_success();
    assert!(response["fee_in_lamports"].as_u64().is_some(), "Expected fee_in_lamports in response");
}

/// The signing path, which is the one that matters most: a failed lookup table
/// resolution or fee-payer balance check here would reject a legitimate
/// transaction.
#[tokio::test]
async fn test_sign_transaction_fails_over() {
    let ctx = crate::ctx().await;

    let allowed_lookup_table_address =
        LookupTableHelper::get_allowed_lookup_table_address().unwrap();

    let v0_transaction = ctx
        .v0_transaction_builder_with_lookup(vec![allowed_lookup_table_address])
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_spl_transfer(
            &USDCMintTestHelper::get_test_usdc_mint_pubkey(),
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            get_fee_for_default_transaction_in_usdc(),
        )
        .with_transfer(
            &SenderTestHelper::get_test_sender_keypair().pubkey(),
            &RecipientTestHelper::get_recipient_pubkey(),
            10,
        )
        .build()
        .await
        .expect("Failed to build V0 transaction");

    let response: serde_json::Value = ctx
        .rpc_call("signTransaction", rpc_params![v0_transaction])
        .await
        .expect("signTransaction should fail over");

    response.assert_success();
    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

/// Failover has to hold across many calls, not just the one that trips it: the
/// dead endpoint is retried in the half-open state on purpose, and each retry
/// that fails must not degrade later requests.
#[tokio::test]
async fn test_failover_holds_across_repeated_requests() {
    let ctx = crate::ctx().await;

    for request in 0..5 {
        let response: serde_json::Value = ctx
            .rpc_call("getBlockhash", rpc_params![])
            .await
            .unwrap_or_else(|e| panic!("request {request} should have failed over: {e}"));

        response.assert_success();
        response.assert_valid_blockhash();
    }
}
