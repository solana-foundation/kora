#![cfg(test)]

use hmac::{Hmac, KeyInit, Mac};
use kora_lib::constant::{X_HMAC_SIGNATURE, X_TIMESTAMP};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sha2::Sha256;

use crate::common::constants::TEST_HMAC_SECRET;

pub static JSON_TEST_BODY: Lazy<Value> = Lazy::new(|| {
    json!({
        "jsonrpc": "2.0",
        "method": "getBlockhash",
        "params": [],
        "id": 1
    })
});

/// Helper to make JSON-RPC request with custom headers to test server
pub async fn make_auth_request(
    server_url: &str,
    headers: Option<Vec<(&str, &str)>>,
) -> reqwest::Response {
    let client = reqwest::Client::new();

    let mut request = client
        .post(server_url)
        .header("Content-Type", "application/json")
        .json(&JSON_TEST_BODY.clone());

    if let Some(custom_headers) = headers {
        for (key, value) in custom_headers {
            request = request.header(key, value);
        }
    }

    request.send().await.expect("Request should complete")
}

pub fn get_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

pub fn create_hmac_signature(secret: &str, timestamp: &str, body: &str) -> String {
    let message = format!("{timestamp}{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn create_valid_hmac_signature_headers() -> Vec<(String, String)> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let signature =
        create_hmac_signature(TEST_HMAC_SECRET, &timestamp, &JSON_TEST_BODY.to_string());

    vec![(X_TIMESTAMP.to_string(), timestamp), (X_HMAC_SIGNATURE.to_string(), signature)]
}
