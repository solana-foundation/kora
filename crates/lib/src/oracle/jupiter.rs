use super::{PriceSource, TokenPrice};
use crate::error::KoraError;
use reqwest::Client;
use serde::Deserialize;

const JUPITER_API_URL: &str = "https://price.jup.ag/v4";

#[derive(Debug, Deserialize)]
struct JupiterResponse {
    data: Vec<JupiterPriceData>,
}

#[derive(Debug, Deserialize)]
struct JupiterPriceData {
    _id: String,
    price: f64,
}

pub async fn get_price(client: &Client, mint_address: &str) -> Result<TokenPrice, KoraError> {
    let url = format!("{}/price?ids={}", JUPITER_API_URL, mint_address);

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
        .first()
        .ok_or_else(|| KoraError::RpcError("No price data from Jupiter".to_string()))?;

    Ok(TokenPrice {
        price: price_data.price,
        confidence: 0.95,
        source: PriceSource::Jupiter,
    })
} 