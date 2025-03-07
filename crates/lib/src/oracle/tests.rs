use super::*;
use mockito::mock;
use tokio;

#[tokio::test]
async fn test_jupiter_price_fetch() {
    let mock_response = r#"{
        "data": {
            "So11111111111111111111111111111111111111112": {
                "id": "So11111111111111111111111111111111111111112",
                "type": "derivedPrice",
                "price": "1"
            },
            "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN": {
                "id": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
                "type": "derivedPrice",
                "price": "0.005321503266927636"
            }
        },
        "timeTaken": 0.003297425
    }"#;

    let _m = mock("GET", "/price")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create();

    let client = Client::new();
    let result = jupiter::get_price(&client, "So11111111111111111111111111111111111111112").await;

    assert!(result.is_ok());
    let price = result.unwrap();
    assert_eq!(price.price, 1.0);
    assert_eq!(price.source, PriceSource::Jupiter);
}

#[tokio::test]
async fn test_price_oracle_retries() {
    let oracle = PriceOracle::new(3, Duration::from_millis(100));

    // Mock failed requests followed by success
    let _m1 = mock("GET", "/price")
        .with_status(500)
        .create();

    let _m2 = mock("GET", "/price")
        .with_status(200)
        .with_body(r#"{
            "data": {
                "test": {
                    "id": "test",
                    "type": "derivedPrice",
                    "price": "1.0"
                }
            },
            "timeTaken": 0.001
        }"#)
        .create();

    let result = oracle.get_token_price("test").await;
    assert!(result.is_ok());
}