use super::{PriceOracle, PriceSource, TokenPrice};
use crate::{
    constant::{JUPITER_API_URL, SOL_MINT},
    error::KoraError,
    sanitize_error,
    validator::math_validator,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

const JUPITER_AUTH_HEADER: &str = "x-api-key";

const JUPITER_DEFAULT_CONFIDENCE: f64 = 0.95;

const MAX_REASONABLE_PRICE: f64 = 1_000_000.0;
const MIN_REASONABLE_PRICE: f64 = 0.000_000_001;

static GLOBAL_JUPITER_API_KEY: Lazy<Arc<RwLock<Option<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global Jupiter API key from the environment variable
pub fn init_jupiter_api_key() {
    let mut api_key_guard = GLOBAL_JUPITER_API_KEY.write();
    if api_key_guard.is_none() {
        *api_key_guard = std::env::var("JUPITER_API_KEY").ok();
    }
}

/// Get the global Jupiter API key, falling back to environment variable
fn get_jupiter_api_key() -> Option<String> {
    let api_key_guard = GLOBAL_JUPITER_API_KEY.read();
    api_key_guard.clone().or_else(|| std::env::var("JUPITER_API_KEY").ok())
}

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
    api_url: String,
    api_key: String,
}

impl JupiterPriceOracle {
    pub fn new() -> Result<Self, KoraError> {
        let api_key = get_jupiter_api_key().ok_or(KoraError::ConfigError)?;

        let api_url = Self::build_price_api_url(JUPITER_API_URL);

        Ok(Self { api_url, api_key })
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
        let prices = self.get_prices(client, &[mint_address.to_string()]).await?;

        prices.get(mint_address).cloned().ok_or_else(|| {
            KoraError::RpcError(format!("No price data from Jupiter for mint {mint_address}"))
        })
    }

    async fn get_prices(
        &self,
        client: &Client,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        self.fetch_prices_from_url(client, &self.api_url, mint_addresses, &self.api_key).await
    }
}

impl JupiterPriceOracle {
    fn validate_price_data(price_data: &JupiterPriceData, mint: &str) -> Result<(), KoraError> {
        let price = price_data.usd_price;

        math_validator::validate_division(price)?;

        // Sanity check: price should be within reasonable bounds
        if price > MAX_REASONABLE_PRICE {
            log::error!(
                "Price data for mint {} exceeds reasonable bounds: {} > {}",
                mint,
                price,
                MAX_REASONABLE_PRICE
            );
            return Err(KoraError::RpcError(format!(
                "Price data for mint {} exceeds reasonable bounds",
                mint
            )));
        }

        if price < MIN_REASONABLE_PRICE {
            log::error!(
                "Price data for mint {} below reasonable bounds: {} < {}",
                mint,
                price,
                MIN_REASONABLE_PRICE
            );
            return Err(KoraError::RpcError(format!(
                "Price data for mint {} below reasonable bounds",
                mint
            )));
        }

        Ok(())
    }

    async fn fetch_prices_from_url(
        &self,
        client: &Client,
        api_url: &str,
        mint_addresses: &[String],
        api_key: &str,
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let mut all_mints = vec![SOL_MINT.to_string()];
        all_mints.extend_from_slice(mint_addresses);
        let ids = all_mints.join(",");

        let url = format!("{api_url}?ids={ids}");

        let request = client.get(&url).header(JUPITER_AUTH_HEADER, api_key);

        let response = request.send().await.map_err(|e| {
            KoraError::RpcError(format!("Jupiter API request failed: {}", sanitize_error!(e)))
        })?;

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

        let jupiter_response: JupiterResponse = response.json().await.map_err(|e| {
            KoraError::RpcError(format!("Failed to parse Jupiter response: {}", sanitize_error!(e)))
        })?;

        // Get SOL price for conversion
        let sol_price = jupiter_response
            .get(SOL_MINT)
            .ok_or_else(|| KoraError::RpcError("No SOL price data from Jupiter".to_string()))?;

        Self::validate_price_data(sol_price, SOL_MINT)?;

        // Convert all prices to SOL-denominated
        let mut result = HashMap::new();
        for mint_address in mint_addresses {
            if let Some(price_data) = jupiter_response.get(mint_address.as_str()) {
                Self::validate_price_data(price_data, mint_address)?;

                // Convert f64 USD prices to Decimal at API boundary
                let token_usd =
                    Decimal::from_f64_retain(price_data.usd_price).ok_or_else(|| {
                        KoraError::RpcError(format!("Invalid token price for mint {mint_address}"))
                    })?;
                let sol_usd = Decimal::from_f64_retain(sol_price.usd_price).ok_or_else(|| {
                    KoraError::RpcError("Invalid SOL price from Jupiter".to_string())
                })?;

                let price_in_sol = token_usd / sol_usd;

                result.insert(
                    mint_address.clone(),
                    TokenPrice {
                        price: price_in_sol,
                        confidence: JUPITER_DEFAULT_CONFIDENCE,
                        source: PriceSource::Jupiter,
                    },
                );
            } else {
                log::error!("No price data for mint {mint_address} from Jupiter");
                return Err(KoraError::RpcError(format!(
                    "No price data from Jupiter for mint {mint_address}"
                )));
            }
        }

        if result.is_empty() {
            return Err(KoraError::RpcError("No price data from Jupiter".to_string()));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_new_fails_without_api_key() {
        {
            let mut api_key_guard = GLOBAL_JUPITER_API_KEY.write();
            *api_key_guard = None;
        }

        let result = JupiterPriceOracle::new();
        assert!(result.is_err());
        assert_eq!(result.err(), Some(KoraError::ConfigError));
    }

    #[tokio::test]
    #[serial]
    async fn test_jupiter_price_fetch_with_api_key() {
        {
            let mut api_key_guard = GLOBAL_JUPITER_API_KEY.write();
            *api_key_guard = Some("test-api-key".to_string());
        }

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
        let mut oracle = JupiterPriceOracle::new().unwrap();
        oracle.api_url = format!("{}/price/v3", server.url());

        let result = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await;
        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, Decimal::from(1));
        assert_eq!(price.source, PriceSource::Jupiter);
    }

    #[tokio::test]
    #[serial]
    async fn test_jupiter_missing_price_data_returns_error() {
        {
            let mut api_key_guard = GLOBAL_JUPITER_API_KEY.write();
            *api_key_guard = Some("test-api-key".to_string());
        }

        let no_price_response = r#"{
            "So11111111111111111111111111111111111111112": {
                "usdPrice": 100.0,
                "blockId": 12345,
                "decimals": 9,
                "priceChange24h": 2.5
            }
        }"#;

        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/price/v3")
            .match_header("x-api-key", "test-api-key")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(no_price_response)
            .create();

        let client = Client::new();
        let mut oracle = JupiterPriceOracle::new().unwrap();
        oracle.api_url = format!("{}/price/v3", server.url());

        let result = oracle.get_price(&client, "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").await;
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(KoraError::RpcError(
                "No price data from Jupiter for mint JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
                    .to_string()
            ))
        );
    }
}
