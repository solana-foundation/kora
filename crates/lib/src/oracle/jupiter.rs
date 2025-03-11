use super::{PriceOracle, PriceSource, TokenPrice};
use crate::error::KoraError;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const JUPITER_API_URL: &str = "https://api.jup.ag/price/v2";

#[derive(Debug, Deserialize)]
struct JupiterResponse {
    data: HashMap<String, JupiterPriceData>,
    #[serde(rename = "timeTaken")]
    #[allow(dead_code)]
    time_taken: f64,
}

#[derive(Debug, Deserialize)]
struct JupiterPriceData {
    id: String,
    #[serde(rename = "type")]
    price_type: String,
    price: String,
}

pub struct JupiterPriceOracle;

#[async_trait::async_trait]
impl PriceOracle for JupiterPriceOracle {
    async fn get_price(&self, client: &Client, mint_address: &str) -> Result<TokenPrice, KoraError> {
        // Get price in SOL using vsToken parameter
        let url = format!(
            "{}/price?ids={}&vsToken=So11111111111111111111111111111111111111112",
            std::env::var("JUPITER_API_URL").unwrap_or(JUPITER_API_URL.to_string()), mint_address
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| KoraError::RpcError(format!("Jupiter API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(KoraError::RpcError(format!("Jupiter API error: {}", response.status())));
        }

        let jupiter_response: JupiterResponse = response
            .json()
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to parse Jupiter response: {}", e)))?;

        let price_data = jupiter_response
            .data
            .get(mint_address)
            .ok_or_else(|| KoraError::RpcError("No price data from Jupiter".to_string()))?;

        // Convert price from string to f64
        let price = price_data
            .price
            .parse::<f64>()
            .map_err(|e| KoraError::RpcError(format!("Failed to parse price: {}", e)))?;

        Ok(TokenPrice { price, confidence: 0.95, source: PriceSource::Jupiter })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

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
        let mut server = Server::new_async().await;
        let _m = server.mock("GET", "/price")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create();
        let url = server.url();
        std::env::set_var("JUPITER_API_URL", &url);
    
        let client = Client::new();
        let oracle = JupiterPriceOracle;
        let result = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await;
        println!("result: {:?}", result);

        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, 1.0);
        assert_eq!(price.source, PriceSource::Jupiter);
    }
}
