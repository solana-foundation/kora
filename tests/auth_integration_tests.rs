use kora_lib::constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP};
use testing_utils::*;

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

#[tokio::test]
async fn test_auth_with_params() {
    let valid_hmac = create_valid_hmac_signature_headers_with_body(&JSON_TEST_BODY_WITH_PARAMS);
    let valid_headers_hmac = valid_hmac.iter().map(|(k, v)| (k.as_str(), v.as_str()));

    let response = make_auth_request_with_body(
        &JSON_TEST_BODY_WITH_PARAMS,
        Some(
            std::iter::once((X_API_KEY, TEST_API_KEY))
                .chain(valid_headers_hmac.clone())
                .collect::<Vec<(&str, &str)>>(),
        ),
    )
    .await;

    assert!(
        response.status().is_success(),
        "Valid API key with params should return 200, got {}",
        response.status()
    );
}
