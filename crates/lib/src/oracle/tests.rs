use super::*;
use mockito::{mock, server_url};
use tokio;

#[tokio::test]
async fn test_price_oracle_consensus() {
    let oracle = PriceOracle::new(3, Duration::from_millis(100));
    
    let prices = vec![
        TokenPrice {
            price: 1.0,
            confidence: 0.9,
            source: PriceSource::Jupiter,
        },
        TokenPrice {
            price: 1.1,
            confidence: 0.95,
            source: PriceSource::Pyth,
        },
        TokenPrice {
            price: 0.9,
            confidence: 0.85,
            source: PriceSource::Jupiter,
        },
        TokenPrice {
            price: 5.0, // Outlier
            confidence: 0.5,
            source: PriceSource::Jupiter,
        },
    ];

    let result = oracle.calculate_consensus_price(prices);
    assert!((result.price - 1.0).abs() < 0.2); // Should be close to 1.0
    assert!(result.confidence > 0.8);
}

#[tokio::test]
async fn test_jupiter_price_fetch() {
    let mut server = mockito::Server::new();
    
    let mock_response = r#"{
        "data": [{
            "id": "So11111111111111111111111111111111111111112",
            "price": 1.5
        }]
    }"#;

    let _m = server.mock("GET", "/price")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_response)
        .create();

    let client = Client::new();
    let result = jupiter::get_price(&client, "So11111111111111111111111111111111111111112").await;
    
    assert!(result.is_ok());
    let price = result.unwrap();
    assert_eq!(price.price, 1.5);
    assert_eq!(price.source, PriceSource::Jupiter);
}

#[tokio::test]
async fn test_price_oracle_retries() {
    let oracle = PriceOracle::new(3, Duration::from_millis(100));
    let mut server = mockito::Server::new();

    // First two requests fail, third succeeds
    let _m1 = server.mock("GET", "/price")
        .with_status(500)
        .create();

    let _m2 = server.mock("GET", "/price")
        .with_status(500)
        .create();

    let _m3 = server.mock("GET", "/price")
        .with_status(200)
        .with_body(r#"{"data":[{"id":"test","price":1.0}]}"#)
        .create();

    let result = oracle.get_token_price("test").await;
    assert!(result.is_ok());
} 