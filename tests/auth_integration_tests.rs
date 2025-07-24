use hmac::{Hmac, Mac};
use kora_lib::constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sha2::Sha256;
use testing_utils::*;

const TEST_API_KEY: &str = "test-api-key-123";
const TEST_HMAC_SECRET: &str = "test-hmac-secret-456";

pub static JSON_TEST_BODY: Lazy<Value> = Lazy::new(|| {
    json!({
        "jsonrpc": "2.0",
        "method": "getBlockhash",
        "params": [],
        "id": 1
    })
});

/// Helper to make JSON-RPC request with custom headers to test server
async fn make_auth_request(headers: Option<Vec<(&str, &str)>>) -> reqwest::Response {
    let client = reqwest::Client::new();

    let mut request = client
        .post(get_test_server_url())
        .header("Content-Type", "application/json")
        .json(&JSON_TEST_BODY.clone());

    if let Some(custom_headers) = headers {
        for (key, value) in custom_headers {
            request = request.header(key, value);
        }
    }

    request.send().await.expect("Request should complete")
}

fn get_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

/// Helper to create HMAC signature
fn create_hmac_signature(secret: &str, timestamp: &str, body: &str) -> String {
    let message = format!("{timestamp}{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn create_valid_hmac_signature_headers() -> Vec<(String, String)> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let signature =
        create_hmac_signature(TEST_HMAC_SECRET, &timestamp, &JSON_TEST_BODY.to_string());

    vec![(X_TIMESTAMP.to_string(), timestamp), (X_HMAC_SIGNATURE.to_string(), signature)]
}

/// Test API key authentication
#[tokio::test]
async fn test_api_key_authentication() {
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

    // Test invalid API key
    let invalid_response = make_auth_request(Some(
        std::iter::once((X_API_KEY, "wrong-key"))
            .chain(valid_headers_hmac.clone())
            .collect::<Vec<(&str, &str)>>(),
    ))
    .await;

    assert_eq!(invalid_response.status(), 401, "Invalid API key should return 401");

    // Test missing API key
    let missing_response =
        make_auth_request(Some(valid_headers_hmac.clone().collect::<Vec<(&str, &str)>>())).await;

    assert_eq!(missing_response.status(), 401, "Missing API key should return 401");
}

#[tokio::test]
async fn test_hmac_authentication() {
    let valid_hmac = create_valid_hmac_signature_headers();
    let valid_headers_hmac = valid_hmac.iter().map(|(k, v)| (k.as_str(), v.as_str()));

    let response = make_auth_request(Some(
        std::iter::once((X_API_KEY, TEST_API_KEY))
            .chain(valid_headers_hmac.clone())
            .collect::<Vec<(&str, &str)>>(),
    ))
    .await;

    assert!(
        response.status().is_success(),
        "Valid HMAC should return 200, got {}",
        response.status()
    );

    // Test invalid HMAC signature
    let invalid_response = make_auth_request(Some(vec![
        (X_API_KEY, TEST_API_KEY),
        (X_HMAC_SIGNATURE, "invalid-signature"),
        (X_TIMESTAMP, get_timestamp().as_str()),
    ]))
    .await;

    assert_eq!(invalid_response.status(), 401, "Invalid HMAC should return 401");

    // Test expired timestamp
    let expired_timestamp =
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
            - 600) // 10 minutes ago
            .to_string();

    let expired_signature =
        create_hmac_signature(TEST_HMAC_SECRET, &expired_timestamp, &JSON_TEST_BODY.to_string());

    let expired_response = make_auth_request(Some(vec![
        (X_API_KEY, TEST_API_KEY),
        (X_TIMESTAMP, expired_timestamp.as_str()),
        (X_HMAC_SIGNATURE, expired_signature.as_str()),
    ]))
    .await;

    assert_eq!(expired_response.status(), 401, "Expired timestamp should return 401");
}

#[tokio::test]
async fn test_liveness_bypass() {
    let client = reqwest::Client::new();
    let liveness_response = client
        .get(format!("{}/liveness", get_test_server_url()))
        .send()
        .await
        .expect("Liveness request should succeed");

    assert!(
        liveness_response.status().is_success(),
        "Liveness should bypass auth, got {}",
        liveness_response.status()
    );
}
