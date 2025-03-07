use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::error::KoraError;

pub mod fake;
pub mod jupiter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub price: f64,
    pub confidence: f64,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriceSource {
    Jupiter,
    Fake,
}

pub struct PriceOracle {
    client: Client,
    max_retries: u32,
    base_delay: Duration,
    price_source: PriceSource,
}

impl PriceOracle {
    pub fn new(max_retries: u32, base_delay: Duration, price_source: PriceSource) -> Self {
        Self { client: Client::new(), max_retries, base_delay, price_source }
    }

    pub async fn get_token_price(&self, mint_address: &str) -> Result<TokenPrice, KoraError> {
        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..self.max_retries {
            let price_result = match self.price_source {
                PriceSource::Jupiter => jupiter::get_price(&self.client, mint_address).await,
                PriceSource::Fake => fake::get_price(&self.client, mint_address).await,
            };

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
