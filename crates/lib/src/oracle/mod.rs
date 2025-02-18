use std::time::Duration;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use reqwest::Client;

use crate::error::KoraError;

pub mod jupiter;
pub mod pyth;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub price: f64,
    pub confidence: f64,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSource {
    Jupiter,
    Pyth,
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
            match self.fetch_prices(mint_address).await {
                Ok(prices) => {
                    // Get median price excluding outliers
                    return Ok(self.calculate_consensus_price(prices));
                }
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

    async fn fetch_prices(&self, mint_address: &str) -> Result<Vec<TokenPrice>, KoraError> {
        let mut prices = Vec::new();

        // Get prices from multiple sources concurrently
        let (jupiter_result, pyth_result) = tokio::join!(
            jupiter::get_price(&self.client, mint_address),
            pyth::get_price(&self.client, mint_address)
        );

        if let Ok(price) = jupiter_result {
            prices.push(price);
        }

        if let Ok(price) = pyth_result {
            prices.push(price);
        }

        if prices.is_empty() {
            return Err(KoraError::InternalServerError(
                "No valid price sources available".to_string(),
            ));
        }

        Ok(prices)
    }

    fn calculate_consensus_price(&self, mut prices: Vec<TokenPrice>) -> TokenPrice {
        prices.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        
        // Remove outliers (prices outside 1.5 IQR)
        if prices.len() >= 4 {
            let q1_idx = prices.len() / 4;
            let q3_idx = 3 * prices.len() / 4;
            let iqr = prices[q3_idx].price - prices[q1_idx].price;
            let lower_bound = prices[q1_idx].price - 1.5 * iqr;
            let upper_bound = prices[q3_idx].price + 1.5 * iqr;

            prices.retain(|p| p.price >= lower_bound && p.price <= upper_bound);
        }

        // Calculate weighted average based on confidence
        let (sum_weighted_prices, sum_weights) = prices.iter().fold((0.0, 0.0), |acc, price| {
            (acc.0 + price.price * price.confidence, acc.1 + price.confidence)
        });

        let consensus_price = sum_weighted_prices / sum_weights;
        let avg_confidence = sum_weights / prices.len() as f64;

        // Use the source with highest confidence
        let best_source = prices
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|p| p.source.clone())
            .unwrap_or(PriceSource::Jupiter);

        TokenPrice {
            price: consensus_price,
            confidence: avg_confidence,
            source: best_source,
        }
    }
} 