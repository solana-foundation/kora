use crate::common::*;
use serde_json::json;

/// Test that oversized request bodies are rejected with 413 Payload Too Large
#[tokio::test]
async fn test_request_body_oversized_rejected() {
    // Create a very large JSON payload (4 MB, exceeds 2 MB limit)
    let large_string = "x".repeat(4 * 1024 * 1024);

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "signTransaction",
        "params": {
            "transaction": large_string,
        }
    });

    let client = reqwest::Client::new();
    let response =
        client.post(TestClient::get_default_server_url()).json(&request_body).send().await;

    // Should get 413 Payload Too Large
    assert_eq!(
        response.unwrap().status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "Oversized request should return 413 Payload Too Large"
    );
}
