use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::error::KoraError;

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
}

pub struct PriceOracle {
    client: Client,
    max_retries: u32,
    base_delay: Duration,
}

impl PriceOracle {
    pub fn new(max_retries: u32, base_delay: Duration) -> Self {
        Self {
            client: Client::new(),
            max_retries,
            base_delay,
        }
    }

    pub async fn get_token_price(&self, mint_address: &str) -> Result<TokenPrice, KoraError> {
        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..self.max_retries {
            match jupiter::get_price(&self.client, mint_address).await {
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