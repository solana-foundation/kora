use crate::common::{auth_helpers::create_valid_hmac_signature_headers, *};
use kora_lib::constant::X_API_KEY;

/// Test API key authentication with valid key
#[tokio::test]
async fn test_api_key_authentication_valid() {
    let valid_hmac = create_valid_hmac_signature_headers();
    let valid_headers_hmac = valid_hmac.iter().map(|(k, v)| (k.as_str(), v.as_str()));

    // Test valid API key
    let response = make_auth_request(Some(
        std::iter::once((X_API_KEY, TEST_API_KEY))
            .chain(valid_headers_hmac.clone())
            .collect::<Vec<(&str, &str)>>(),
    ))
    .await;

    assert!(
        response.status().is_success(),
        "Valid API key should return 200, got {}",
        response.status()
    );
}

/// Test API key authentication with invalid key (should fail)
#[tokio::test]
async fn test_api_key_authentication_invalid() {
    let valid_hmac = create_valid_hmac_signature_headers();
    let valid_headers_hmac = valid_hmac.iter().map(|(k, v)| (k.as_str(), v.as_str()));

    // Test invalid API key
    let invalid_response = make_auth_request(Some(
        std::iter::once((X_API_KEY, "wrong-key"))
            .chain(valid_headers_hmac.clone())
            .collect::<Vec<(&str, &str)>>(),
    ))
    .await;

    assert_eq!(invalid_response.status(), 401, "Invalid API key should return 401");
}

/// Test API key authentication with missing key (should fail)
#[tokio::test]
async fn test_api_key_authentication_missing() {
    let valid_hmac = create_valid_hmac_signature_headers();
    let valid_headers_hmac = valid_hmac.iter().map(|(k, v)| (k.as_str(), v.as_str()));

    // Test missing API key
    let missing_response =
        make_auth_request(Some(valid_headers_hmac.clone().collect::<Vec<(&str, &str)>>())).await;

    assert_eq!(missing_response.status(), 401, "Missing API key should return 401");
}

/// Test that liveness endpoint bypasses API key authentication
#[tokio::test]
async fn test_liveness_bypasses_api_key_auth() {
    let client = reqwest::Client::new();
    let liveness_response = client
        .get(format!("{}/liveness", TestClient::get_default_server_url()))
        .send()
        .await
        .expect("Liveness request should succeed");

    assert!(
        liveness_response.status().is_success(),
        "Liveness should bypass auth, got {}",
        liveness_response.status()
    );
}
