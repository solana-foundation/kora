use super::{PriceSource, TokenPrice};
use crate::error::KoraError;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const JUPITER_API_URL: &str = "https://api.jup.ag/price/v2";

#[derive(Debug, Deserialize)]
struct JupiterResponse {
    data: HashMap<String, JupiterPriceData>,
    #[serde(rename = "timeTaken")]
    time_taken: f64,
}

#[derive(Debug, Deserialize)]
struct JupiterPriceData {
    id: String,
    #[serde(rename = "type")]
    price_type: String,
    price: String,
}

pub async fn get_price(client: &Client, mint_address: &str) -> Result<TokenPrice, KoraError> {
    // Get price in SOL using vsToken parameter
    let url = format!(
        "{}/price?ids={}&vsToken=So11111111111111111111111111111111111111112",
        JUPITER_API_URL, mint_address
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
