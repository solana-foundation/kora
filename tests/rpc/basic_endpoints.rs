use crate::common::*;
use jsonrpsee::rpc_params;
use serde_json::json;

/// Test getSupportedTokens endpoint
#[tokio::test]
async fn test_get_supported_tokens() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value = ctx
        .rpc_call("getSupportedTokens", rpc_params![])
        .await
        .expect("Failed to get supported tokens");

    response.assert_success();
    response.assert_has_field("tokens");

    let tokens = response
        .get_field("tokens")
        .expect("Missing tokens field")
        .as_array()
        .expect("Expected tokens array");

    assert!(!tokens.is_empty(), "Tokens list should not be empty");

    // Check for specific known tokens
    let expected_token = USDCMintTestHelper::get_test_usdc_mint_pubkey().to_string();
    assert!(
        tokens.contains(&json!(expected_token)),
        "Expected USDC token {expected_token} not found"
    );
}

/// Test getBlockhash endpoint
#[tokio::test]
async fn test_get_blockhash() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value =
        ctx.rpc_call("getBlockhash", rpc_params![]).await.expect("Failed to get blockhash");

    response.assert_success();
    response.assert_has_field("blockhash");
    response.assert_valid_blockhash();
}

/// Test getConfig endpoint
#[tokio::test]
async fn test_get_config() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    response.assert_success();
    response.assert_has_field("fee_payers");
    response.assert_has_field("validation_config");

    // Specific validations for config structure
    let fee_payers = response
        .get_field("fee_payers")
        .and_then(|fp| fp.as_array())
        .expect("Expected fee_payers array in response");

    assert!(!fee_payers.is_empty(), "Expected at least one fee payer");

    let validation_config = response
        .get_field("validation_config")
        .and_then(|vc| vc.as_object())
        .expect("Expected validation_config object in response");

    assert!(!validation_config.is_empty(), "Expected validation_config to have properties");
}

/// Test getPayerSigner endpoint
#[tokio::test]
async fn test_get_payer_signer() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let response: serde_json::Value =
        ctx.rpc_call("getPayerSigner", rpc_params![]).await.expect("Failed to get payer signer");

    response.assert_success();
    response.assert_has_field("signer_address");
    response.assert_has_field("payment_address");

    // Validate the addresses are valid pubkey strings
    let signer_address = response
        .get_field("signer_address")
        .and_then(|sa| sa.as_str())
        .expect("Expected signer_address in response");

    let payment_address = response
        .get_field("payment_address")
        .and_then(|pa| pa.as_str())
        .expect("Expected payment_address in response");

    // Basic validation - should be valid pubkey format (44 chars base58)
    assert_eq!(signer_address.len(), 44, "Signer address should be 44 chars");
    assert_eq!(payment_address.len(), 44, "Payment address should be 44 chars");
}

/// Test fee payer policy is present in config
#[tokio::test]
async fn test_fee_payer_policy_is_present() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let config_response: serde_json::Value =
        ctx.rpc_call("getConfig", rpc_params![]).await.expect("Failed to get config");

    config_response.assert_success();
    config_response.assert_has_field("validation_config");

    let validation_config = config_response["validation_config"]
        .as_object()
        .expect("Expected validation_config in response");

    let fee_payer_policy = validation_config["fee_payer_policy"]
        .as_object()
        .expect("Expected fee_payer_policy in validation_config");

    // Validate nested policy structure
    assert!(fee_payer_policy.contains_key("system"));
    assert!(fee_payer_policy.contains_key("spl_token"));
    assert!(fee_payer_policy.contains_key("token_2022"));

    // Validate system policy structure
    let system = fee_payer_policy["system"].as_object().expect("Expected system policy object");
    assert!(system.contains_key("allow_transfer"));
    assert!(system.contains_key("allow_assign"));
    assert!(system.contains_key("allow_create_account"));
    assert!(system.contains_key("allow_allocate"));
    assert!(system.contains_key("nonce"));
    assert_eq!(system["allow_transfer"], true);
    assert_eq!(system["allow_assign"], true);
    assert_eq!(system["allow_create_account"], true);
    assert_eq!(system["allow_allocate"], true);

    // Validate nonce nested policy
    let nonce = system["nonce"].as_object().expect("Expected nonce policy object");
    assert!(nonce.contains_key("allow_initialize"));
    assert!(nonce.contains_key("allow_advance"));
    assert!(nonce.contains_key("allow_withdraw"));
    assert!(nonce.contains_key("allow_authorize"));
    assert!(!nonce.contains_key("allow_upgrade"), "allow_upgrade should not exist");
    assert_eq!(nonce["allow_initialize"], true);
    assert_eq!(nonce["allow_advance"], true);
    assert_eq!(nonce["allow_withdraw"], true);
    assert_eq!(nonce["allow_authorize"], true);

    // Validate spl_token policy structure
    let spl_token =
        fee_payer_policy["spl_token"].as_object().expect("Expected spl_token policy object");
    assert!(spl_token.contains_key("allow_transfer"));
    assert!(spl_token.contains_key("allow_burn"));
    assert!(spl_token.contains_key("allow_close_account"));
    assert!(spl_token.contains_key("allow_approve"));
    assert_eq!(spl_token["allow_transfer"], true);
    assert_eq!(spl_token["allow_burn"], true);
    assert_eq!(spl_token["allow_close_account"], true);
    assert_eq!(spl_token["allow_approve"], true);

    // Validate token_2022 policy structure
    let token_2022 =
        fee_payer_policy["token_2022"].as_object().expect("Expected token_2022 policy object");
    assert!(token_2022.contains_key("allow_transfer"));
    assert!(token_2022.contains_key("allow_burn"));
    assert!(token_2022.contains_key("allow_close_account"));
    assert!(token_2022.contains_key("allow_approve"));
    assert_eq!(token_2022["allow_transfer"], true);
    assert_eq!(token_2022["allow_burn"], true);
    assert_eq!(token_2022["allow_close_account"], true);
    assert_eq!(token_2022["allow_approve"], true);
}

/// Test that liveness endpoint is disabled (returns error)
#[tokio::test]
async fn test_liveness_is_disabled() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    // With MethodValidationLayer, disabled methods return 405 METHOD_NOT_ALLOWED at middleware level
    // before reaching jsonrpsee's method dispatcher
    let result = ctx.rpc_call::<serde_json::Value, _>("liveness", rpc_params![]).await;
    assert!(result.is_err());
    let error_msg = result.err().unwrap().to_string();
    // The error should be HTTP 405 (caught by MethodValidationLayer middleware)
    assert!(error_msg.contains("405"), "Expected 405 METHOD_NOT_ALLOWED, got: {}", error_msg);
}
