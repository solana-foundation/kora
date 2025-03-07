use super::{PriceSource, TokenPrice};
use crate::error::KoraError;
use reqwest::Client;

/// A fake price oracle implementation that always returns a fixed price
/// regardless of the token mint address.
/// 
/// This is useful for testing and development environments where
/// real price data is not needed or available.
pub async fn get_price(_client: &Client, _mint_address: &str) -> Result<TokenPrice, KoraError> {
    // Always return a fixed price of 150 with high confidence
    // The source is marked as "Fake" to clearly indicate this is not real market data
    Ok(TokenPrice {
        price: 150.0,
        confidence: 1.0,
        source: PriceSource::Fake
    })
}
