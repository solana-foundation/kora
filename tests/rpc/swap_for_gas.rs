use crate::common::*;
use jsonrpsee::rpc_params;
use kora_lib::{constant::DEFAULT_SWAP_FOR_GAS_SPREAD_BPS, transaction::TransactionUtil};
use solana_sdk::{pubkey::Pubkey, signature::Signature, signer::Signer};
use std::str::FromStr;

#[tokio::test]
async fn test_sign_swap_for_gas_builds_kora_signed_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_keypair = SenderTestHelper::get_test_sender_keypair();
    let source_wallet = source_keypair.pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let lamports_out = 10_000u64;

    let response: serde_json::Value = ctx
        .rpc_call(
            "signSwapForGas",
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
        .expect("Failed to build signSwapForGas transaction");

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
        .expect("Failed to decode signSwapForGas transaction");

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
async fn test_sign_and_send_swap_for_gas_after_user_signature() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_keypair = SenderTestHelper::get_test_sender_keypair();
    let source_wallet = source_keypair.pubkey();
    let destination_wallet = RecipientTestHelper::get_recipient_pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let signed_by_kora_response: serde_json::Value = ctx
        .rpc_call(
            "signSwapForGas",
            rpc_params![
                source_wallet.to_string(),
                Some(destination_wallet.to_string()),
                fee_token.to_string(),
                25_000u64,
                Option::<String>::None,
                false
            ],
        )
        .await
        .expect("Failed to build signSwapForGas transaction");

    signed_by_kora_response.assert_success();
    let signer_pubkey = signed_by_kora_response["signer_pubkey"]
        .as_str()
        .expect("Missing signer_pubkey")
        .to_string();

    let mut tx = TransactionUtil::decode_b64_transaction(
        signed_by_kora_response["transaction"].as_str().expect("Missing transaction"),
    )
    .expect("Failed to decode signSwapForGas transaction");

    let message_bytes = tx.message.serialize();
    let source_index = tx
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == &source_wallet)
        .expect("Source wallet not found in account keys");
    tx.signatures[source_index] = source_keypair.sign_message(&message_bytes);

    let fully_signed = TransactionUtil::encode_versioned_transaction(&tx)
        .expect("Failed to encode fully signed transaction");

    let result: serde_json::Value = ctx
        .rpc_call(
            "signAndSendSwapForGas",
            rpc_params![fully_signed, Some(signer_pubkey), false, Option::<String>::None],
        )
        .await
        .expect("signAndSendSwapForGas should succeed");

    result.assert_success();
    result.assert_has_field("signed_transaction");
    result.assert_has_field("signer_pubkey");
    result.assert_has_field("signature");
}

#[tokio::test]
async fn test_sign_and_send_swap_for_gas_signs_when_kora_signature_missing() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_keypair = SenderTestHelper::get_test_sender_keypair();
    let source_wallet = source_keypair.pubkey();
    let destination_wallet = RecipientTestHelper::get_recipient_pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let sign_response: serde_json::Value = ctx
        .rpc_call(
            "signSwapForGas",
            rpc_params![
                source_wallet.to_string(),
                Some(destination_wallet.to_string()),
                fee_token.to_string(),
                25_000u64,
                Option::<String>::None,
                false
            ],
        )
        .await
        .expect("Failed to build signSwapForGas transaction");

    sign_response.assert_success();
    let signer_pubkey =
        Pubkey::from_str(sign_response["signer_pubkey"].as_str().expect("Missing signer_pubkey"))
            .expect("Invalid signer_pubkey");

    let mut tx = TransactionUtil::decode_b64_transaction(
        sign_response["transaction"].as_str().expect("Missing transaction"),
    )
    .expect("Failed to decode signSwapForGas transaction");

    let signer_index = tx
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == &signer_pubkey)
        .expect("Signer wallet not found in account keys");
    tx.signatures[signer_index] = Signature::default();

    let message_bytes = tx.message.serialize();
    let source_index = tx
        .message
        .static_account_keys()
        .iter()
        .position(|key| key == &source_wallet)
        .expect("Source wallet not found in account keys");
    tx.signatures[source_index] = source_keypair.sign_message(&message_bytes);

    let partially_signed = TransactionUtil::encode_versioned_transaction(&tx)
        .expect("Failed to encode partially signed transaction");

    let result: serde_json::Value = ctx
        .rpc_call(
            "signAndSendSwapForGas",
            rpc_params![
                partially_signed,
                Some(signer_pubkey.to_string()),
                false,
                Option::<String>::None
            ],
        )
        .await
        .expect("signAndSendSwapForGas should succeed and sign with Kora");

    result.assert_success();
    result.assert_has_field("signed_transaction");
    result.assert_has_field("signer_pubkey");
    result.assert_has_field("signature");
}

#[tokio::test]
async fn test_sign_swap_for_gas_rejects_zero_lamports() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let source_wallet = SenderTestHelper::get_test_sender_keypair().pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signSwapForGas",
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
async fn test_sign_swap_for_gas_rejects_source_wallet_equal_to_kora_signer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let payer_signer: serde_json::Value =
        ctx.rpc_call("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");
    let signer_address =
        payer_signer["signer_address"].as_str().expect("Missing signer_address").to_string();

    let destination_wallet = RecipientTestHelper::get_recipient_pubkey();
    let fee_token = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let result = ctx
        .rpc_call::<serde_json::Value, _>(
            "signSwapForGas",
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
