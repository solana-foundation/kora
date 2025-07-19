#[tokio::test]
async fn test_transfer_transaction() {
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    let app = crate::routes::transfer::routes();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/transferTransaction")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "from": "Alice",
                        "to": "Bob",
                        "amount": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

