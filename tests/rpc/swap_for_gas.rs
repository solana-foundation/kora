use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::{constant::DEFAULT_SWAP_FOR_GAS_SPREAD_BPS, transaction::TransactionUtil};
use solana_sdk::{pubkey::Pubkey, signature::Signature, signer::Signer};
use std::str::FromStr;

#[tokio::test]
async fn test_swap_for_gas_builds_kora_signed_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_keypair = SenderTestHelper::get_test_sender_keypair();
    let source_wallet = source_keypair.pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let lamports_out = 10_000u64;

    let response: serde_json::Value = ctx
        .rpc_call(
            "swapForGas",
            rpc_params![
                source_wallet.to_string(),
                Option::<String>::None,
                fee_token.to_string(),
                lamports_out,
                Option::<String>::None,
                false
            ],
        )
        .await
        .expect("Failed to build swapForGas transaction");

    response.assert_success();
    response.assert_has_field("transaction");
    response.assert_has_field("signer_pubkey");
    response.assert_has_field("payment_address");
    response.assert_has_field("token_amount_in");
    response.assert_has_field("spread_bps");

    assert_eq!(response["destination_wallet"].as_str().unwrap(), source_wallet.to_string());
    assert_eq!(response["fee_token"].as_str().unwrap(), fee_token.to_string());
    assert_eq!(response["lamports_out"].as_u64().unwrap(), lamports_out);
    assert_eq!(
        response["spread_bps"].as_u64().unwrap(),
        u64::from(DEFAULT_SWAP_FOR_GAS_SPREAD_BPS)
    );
    assert!(response["token_amount_in"].as_u64().unwrap() > 0);

    let tx = TransactionUtil::decode_b64_transaction(response["transaction"].as_str().unwrap())
        .expect("Failed to decode swapForGas transaction");

    assert_eq!(tx.signatures.len(), tx.message.header().num_required_signatures as usize);

    let account_keys = tx.message.static_account_keys();
    assert!(account_keys.contains(&source_wallet));

    let signer_pubkey = Pubkey::from_str(response["signer_pubkey"].as_str().unwrap())
        .expect("Invalid signer_pubkey");
    let signer_index = account_keys
        .iter()
        .position(|key| key == &signer_pubkey)
        .expect("Signer pubkey not found in account keys");
    let source_index = account_keys
        .iter()
        .position(|key| key == &source_wallet)
        .expect("Source wallet not found in account keys");
    let required_signers = tx.message.header().num_required_signatures as usize;

    assert!(signer_index < required_signers);
    assert!(source_index < required_signers);
    assert_ne!(tx.signatures[signer_index], Signature::default());
    assert_eq!(tx.signatures[source_index], Signature::default());
}

#[tokio::test]
async fn test_swap_for_gas_rejects_zero_lamports() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_wallet = SenderTestHelper::get_test_sender_keypair().pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "swapForGas",
            rpc_params![
                source_wallet.to_string(),
                Option::<String>::None,
                fee_token.to_string(),
                0u64,
                Option::<String>::None,
                false
            ],
        )
        .await;

    assert!(result.is_err());
    result.unwrap_err().assert_contains_message("lamports_out must be greater than zero");
}

#[tokio::test]
async fn test_swap_for_gas_rejects_source_wallet_equal_to_kora_signer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let payer_signer: serde_json::Value =
        ctx.rpc_call("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");
    let signer_address =
        payer_signer["signer_address"].as_str().expect("Missing signer_address").to_string();

    let destination_wallet = RecipientTestHelper::get_recipient_pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "swapForGas",
            rpc_params![
                signer_address.clone(),
                Some(destination_wallet.to_string()),
                fee_token.to_string(),
                25_000u64,
                Some(signer_address),
                true
            ],
        )
        .await;

    assert!(result.is_err());
    result.unwrap_err().assert_contains_message("source_wallet must not be the Kora fee payer");
}

#[tokio::test]
async fn test_swap_for_gas_rejects_destination_wallet_equal_to_kora_signer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let payer_signer: serde_json::Value =
        ctx.rpc_call("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");
    let signer_address =
        payer_signer["signer_address"].as_str().expect("Missing signer_address").to_string();

    let source_wallet = SenderTestHelper::get_test_sender_keypair().pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "swapForGas",
            rpc_params![
                source_wallet.to_string(),
                Some(signer_address.clone()),
                fee_token.to_string(),
                25_000u64,
                Some(signer_address),
                true
            ],
        )
        .await;

    assert!(result.is_err());
    result
        .unwrap_err()
        .assert_contains_message("destination_wallet must not be the Kora fee payer");
}
