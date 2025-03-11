use std::sync::Arc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use mockall::automock;
use crate::error::KoraError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub price: f64,
    pub confidence: f64,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriceSource {
    Jupiter,
    Mock
}

#[automock]
#[async_trait::async_trait]
pub trait PriceOracle {
    async fn get_price(&self, client: &Client, mint_address: &str) -> Result<TokenPrice, KoraError>;
}

pub struct RetryingPriceOracle {
    client: Client,
    max_retries: u32,
    base_delay: Duration,
    oracle: Arc<dyn PriceOracle + Send + Sync>,
}

pub fn get_price_oracle(source: PriceSource) -> Arc<dyn PriceOracle + Send + Sync> {
    match source {
        PriceSource::Jupiter => Arc::new(crate::oracle::jupiter::JupiterPriceOracle),
        PriceSource::Mock => Arc::new(MockPriceOracle::new()),
    }
}

impl RetryingPriceOracle {
    pub fn new(max_retries: u32, base_delay: Duration, oracle: Arc<dyn PriceOracle + Send + Sync>) -> Self {
        Self {
            client: Client::new(),
            max_retries,
            base_delay,
            oracle,
        }
    }

    pub async fn get_token_price(&self, mint_address: &str) -> Result<TokenPrice, KoraError> {
        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..self.max_retries {
            let price_result = self.oracle.get_price(&self.client, mint_address).await;

            match price_result {
                Ok(price) => return Ok(price),
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
            KoraError::InternalServerError("Failed to fetch token price".to_string())
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
        mock_oracle
            .expect_get_price()
            .times(1)
            .returning(|_, _| Ok(TokenPrice {
                price: 1.0,
                confidence: 0.95,
                source: PriceSource::Jupiter,
            }));

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(100), Arc::new(mock_oracle));
        let result = oracle.get_token_price("test").await;
        assert!(result.is_ok());
    }
}