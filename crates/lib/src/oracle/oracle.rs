use crate::{
    error::KoraError,
    oracle::{jupiter::JupiterPriceOracle, utils::OracleUtil},
};
use mockall::automock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub struct TokenPrice {
    pub price: f64,
    pub confidence: f64,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "docs", derive(utoipa::ToSchema))]
pub enum PriceSource {
    Jupiter,
    Mock,
}

#[automock]
#[async_trait::async_trait]
pub trait PriceOracle {
    async fn get_price(&self, client: &Client, mint_address: &str)
        -> Result<TokenPrice, KoraError>;

    async fn get_prices(
        &self,
        client: &Client,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError>;
}

pub struct RetryingPriceOracle {
    client: Client,
    max_retries: u32,
    base_delay: Duration,
    oracle: Arc<dyn PriceOracle + Send + Sync>,
}

pub fn get_price_oracle(source: PriceSource) -> Arc<dyn PriceOracle + Send + Sync> {
    match source {
        PriceSource::Jupiter => Arc::new(JupiterPriceOracle::new()),
        PriceSource::Mock => OracleUtil::get_mock_oracle_price(),
    }
}

impl RetryingPriceOracle {
    pub fn new(
        max_retries: u32,
        base_delay: Duration,
        oracle: Arc<dyn PriceOracle + Send + Sync>,
    ) -> Self {
        Self { client: Client::new(), max_retries, base_delay, oracle }
    }

    pub async fn get_token_price(&self, mint_address: &str) -> Result<TokenPrice, KoraError> {
        let prices = self.get_token_prices(&[mint_address.to_string()]).await?;

        prices.get(mint_address).cloned().ok_or_else(|| {
            KoraError::InternalServerError("Failed to fetch token price".to_string())
        })
    }

    pub async fn get_token_prices(
        &self,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..self.max_retries {
            let price_result = self.oracle.get_prices(&self.client, mint_addresses).await;

            match price_result {
                Ok(prices) => return Ok(prices),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries - 1 {
                        sleep(delay).await;
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            KoraError::InternalServerError("Failed to fetch token prices".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_price_oracle_retries() {
        let mut mock_oracle = MockPriceOracle::new();
        mock_oracle.expect_get_prices().times(1).returning(|_, mint_addresses| {
            let mut result = HashMap::new();
            for mint in mint_addresses {
                result.insert(
                    mint.clone(),
                    TokenPrice { price: 1.0, confidence: 0.95, source: PriceSource::Jupiter },
                );
            }
            Ok(result)
        });

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(100), Arc::new(mock_oracle));
        let result = oracle.get_token_price("test").await;
        assert!(result.is_ok());
    }
}
