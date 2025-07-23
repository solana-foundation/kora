use super::{PriceOracle, PriceSource, TokenPrice};
use crate::{
    constant::{JUPITER_API_BASE_URL, SOL_MINT},
    error::KoraError,
};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

type JupiterResponse = HashMap<String, JupiterPriceData>;

#[derive(Debug, Deserialize)]
struct JupiterPriceData {
    #[serde(rename = "usdPrice")]
    usd_price: f64,
    #[serde(rename = "blockId")]
    #[allow(dead_code)]
    block_id: u64,
    #[allow(dead_code)]
    decimals: u8,
    #[serde(rename = "priceChange24h")]
    #[allow(dead_code)]
    price_change_24h: f64,
}

pub struct JupiterPriceOracle {
    jupiter_api_url: String,
}

impl JupiterPriceOracle {
    pub fn new(jupiter_api_base_url: Option<String>) -> Self {
        let base_url = jupiter_api_base_url
            .or_else(|| std::env::var("JUPITER_API_URL").ok())
            .unwrap_or_else(|| JUPITER_API_BASE_URL.to_string());

        // Build full API URL by appending /price/v3 to base URL
        let api_url = Self::build_api_url(&base_url);
        Self { jupiter_api_url: api_url }
    }

    fn build_api_url(base_url: &str) -> String {
        let trimmed = base_url.trim_end_matches('/');
        format!("{trimmed}/price/v3")
    }
}

#[async_trait::async_trait]
impl PriceOracle for JupiterPriceOracle {
    async fn get_price(
        &self,
        client: &Client,
        mint_address: &str,
    ) -> Result<TokenPrice, KoraError> {
        // Always fetch SOL price as well so we can convert to SOL
        let url = format!("{}?ids={},{}", self.jupiter_api_url, SOL_MINT, mint_address);

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| KoraError::RpcError(format!("Jupiter API request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(KoraError::RpcError(format!("Jupiter API error: {}", response.status())));
        }

        let jupiter_response: JupiterResponse = response
            .json()
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to parse Jupiter response: {e}")))?;

        let sol_price = jupiter_response
            .get(SOL_MINT)
            .ok_or_else(|| KoraError::RpcError("No SOL price data from Jupiter".to_string()))?;
        let price_data = jupiter_response
            .get(mint_address)
            .ok_or_else(|| KoraError::RpcError("No price data from Jupiter".to_string()))?;

        let price = price_data.usd_price / sol_price.usd_price;

        Ok(TokenPrice {
            price,
            confidence: 0.95,
            source: PriceSource::Jupiter { api_url: Some(self.jupiter_api_url.clone()) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn test_jupiter_price_fetch() {
        // Jupiter Price API v3 response format
        let mock_response = r#"{
            "So11111111111111111111111111111111111111112": {
                "usdPrice": 100.0,
                "blockId": 12345,
                "decimals": 9,
                "priceChange24h": 2.5
            },
            "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN": {
                "usdPrice": 0.532,
                "blockId": 12345,
                "decimals": 6,
                "priceChange24h": -1.2
            }
        }"#;
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/price/v3")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create();

        let client = Client::new();
        let oracle = JupiterPriceOracle::new(Some(server.url()));
        let result = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await;

        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, 1.0);
        assert_eq!(
            price.source,
            PriceSource::Jupiter { api_url: Some(format!("{}/price/v3", server.url())) }
        );
    }

    #[test]
    fn test_jupiter_url_priority() {
        // Test that config URL takes priority over env var
        std::env::set_var("JUPITER_API_URL", "http://env.example.com");

        let oracle_config = JupiterPriceOracle::new(Some("http://config.example.com".to_string()));
        assert_eq!(oracle_config.jupiter_api_url, "http://config.example.com/price/v3");

        // Test env var fallback when config is None
        let oracle_env = JupiterPriceOracle::new(None);
        assert_eq!(oracle_env.jupiter_api_url, "http://env.example.com/price/v3");

        std::env::remove_var("JUPITER_API_URL");

        // Test default fallback when both config and env are None
        let oracle_default = JupiterPriceOracle::new(None);
        assert_eq!(oracle_default.jupiter_api_url, "https://api.jup.ag/price/v3");

        // Test URL trimming (removes trailing slashes)
        let oracle_with_slash =
            JupiterPriceOracle::new(Some("https://custom-api.com/".to_string()));
        assert_eq!(oracle_with_slash.jupiter_api_url, "https://custom-api.com/price/v3");
    }
}
