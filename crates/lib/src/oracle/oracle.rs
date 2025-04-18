use crate::error::KoraError;
use mockall::automock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub price: f64,
    pub confidence: f64,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriceSource {
    Jupiter,
    Mock,
}

#[automock]
#[async_trait::async_trait]
pub trait PriceOracle {
    async fn get_price(&self, client: &Client, mint_address: &str)
        -> Result<TokenPrice, KoraError>;
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
        PriceSource::Mock => {
            let mut mock = MockPriceOracle::new();
            // Set up default mock behavior for devnet tokens
            mock.expect_get_price()
                .times(..) // Allow unlimited calls
                .returning(|_, mint_address| {
                    const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
                    let price = match mint_address {
                        USDC_DEVNET_MINT => 0.0001, // USDC
                        "So11111111111111111111111111111111111111112" => 1.0,    // SOL
                        _ => 0.001, // Default price for unknown tokens
                    };
                    Ok(TokenPrice {
                        price,
                        confidence: 1.0,
                        source: PriceSource::Mock,
                    })
                });
            Arc::new(mock)
        }
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
        mock_oracle.expect_get_price().times(1).returning(|_, _| {
            Ok(TokenPrice { price: 1.0, confidence: 0.95, source: PriceSource::Jupiter })
        });

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(100), Arc::new(mock_oracle));
        let result = oracle.get_token_price("test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_oracle_prices() {
        let oracle = get_price_oracle(PriceSource::Mock);
        let client = Client::new();

        // Test USDC price
        let usdc_price = oracle.get_price(&client, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").await.unwrap();
        assert_eq!(usdc_price.price, 0.0001);
        assert_eq!(usdc_price.confidence, 1.0);
        assert_eq!(usdc_price.source, PriceSource::Mock);

        // Test SOL price
        let sol_price = oracle.get_price(&client, "So11111111111111111111111111111111111111112").await.unwrap();
        assert_eq!(sol_price.price, 1.0);
        assert_eq!(sol_price.confidence, 1.0);
        assert_eq!(sol_price.source, PriceSource::Mock);

        // Test unknown token (should return default price)
        let unknown_price = oracle.get_price(&client, "unknown_token").await.unwrap();
        assert_eq!(unknown_price.price, 0.001);
        assert_eq!(unknown_price.confidence, 1.0);
        assert_eq!(unknown_price.source, PriceSource::Mock);
    }
}
