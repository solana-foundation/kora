use super::{PriceOracle, PriceSource, TokenPrice};
use crate::{
    constant::{JUPITER_API_LITE_URL, JUPITER_API_PRO_URL, SOL_MINT},
    error::KoraError,
};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;

const JUPITER_AUTH_HEADER: &str = "x-api-key";

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
    price_change_24h: Option<f64>,
}

pub struct JupiterPriceOracle {
    pro_api_url: String,
    lite_api_url: String,
    api_key: Option<String>,
}

impl JupiterPriceOracle {
    pub fn new(api_key: Option<String>) -> Self {
        // Check environment variable for API key
        let api_key = api_key.or_else(|| std::env::var("JUPITER_API_KEY").ok());

        let pro_api_url = Self::build_price_api_url(JUPITER_API_PRO_URL);
        let lite_api_url = Self::build_price_api_url(JUPITER_API_LITE_URL);

        Self { pro_api_url, lite_api_url, api_key }
    }

    fn build_price_api_url(base_url: &str) -> String {
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
        // Try pro API first if API key is available, then fallback to free API
        if let Some(api_key) = &self.api_key {
            match self
                .fetch_price_from_url(client, &self.pro_api_url, mint_address, Some(api_key))
                .await
            {
                Ok(price) => return Ok(price),
                Err(e) => {
                    if e == KoraError::RateLimitExceeded {
                        log::warn!("Pro Jupiter API rate limit exceeded, falling back to free API");
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // Use free API (either as fallback or primary if no API key)
        self.fetch_price_from_url(client, &self.lite_api_url, mint_address, None).await
    }
}

impl JupiterPriceOracle {
    async fn fetch_price_from_url(
        &self,
        client: &Client,
        api_url: &str,
        mint_address: &str,
        api_key: Option<&String>,
    ) -> Result<TokenPrice, KoraError> {
        // Always fetch SOL price as well so we can convert to SOL
        let url = format!("{api_url}?ids={SOL_MINT},{mint_address}");

        let mut request = client.get(&url);

        // Add API key header if provided
        if let Some(key) = api_key {
            request = request.header(JUPITER_AUTH_HEADER, key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| KoraError::RpcError(format!("Jupiter API request failed: {e}")))?;

        if !response.status().is_success() {
            match response.status() {
                StatusCode::TOO_MANY_REQUESTS => {
                    return Err(KoraError::RateLimitExceeded);
                }
                _ => {
                    return Err(KoraError::RpcError(format!(
                        "Jupiter API error: {}",
                        response.status()
                    )));
                }
            }
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

        Ok(TokenPrice { price, confidence: 0.95, source: PriceSource::Jupiter })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn test_jupiter_price_fetch_without_api_key() {
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
        // Test without API key - should use lite API
        let mut oracle = JupiterPriceOracle::new(None);
        oracle.lite_api_url = format!("{}/price/v3", server.url());

        let result = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await;

        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, 1.0);
        assert_eq!(price.source, PriceSource::Jupiter);
    }

    #[tokio::test]
    async fn test_jupiter_price_fetch_with_api_key() {
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
            .match_header("x-api-key", "test-api-key")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response)
            .create();

        let client = Client::new();
        // Test with API key - should use pro API
        let mut oracle = JupiterPriceOracle::new(Some("test-api-key".to_string()));
        oracle.pro_api_url = format!("{}/price/v3", server.url());

        let result = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await;

        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, 1.0);
        assert_eq!(price.source, PriceSource::Jupiter);
    }

    #[test]
    fn test_jupiter_api_key_priority() {
        // Test that config URL takes priority over env var
        std::env::set_var("JUPITER_API_KEY", "321");

        let oracle_config = JupiterPriceOracle::new(Some("123".to_string()));
        assert_eq!(oracle_config.api_key, Some("123".to_string()));

        // Test env var fallback when config is None
        let oracle_env = JupiterPriceOracle::new(None);
        assert_eq!(oracle_env.api_key, Some("321".to_string()));

        std::env::remove_var("JUPITER_API_KEY");

        // Test default fallback when both config and env are None
        let oracle_default = JupiterPriceOracle::new(None);
        assert_eq!(oracle_default.api_key, None);
    }
}
