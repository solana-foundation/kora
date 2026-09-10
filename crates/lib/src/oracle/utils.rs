use crate::{
    error::KoraError,
    oracle::{PriceOracle, PriceSource, TokenPrice},
};
use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_MOCKED_PRICE: Decimal = dec!(0.001);
pub const DEFAULT_MOCKED_USDC_PRICE: Decimal = dec!(0.0075);
pub const DEFAULT_MOCKED_WSOL_PRICE: Decimal = dec!(1.0);

pub const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
pub const WSOL_DEVNET_MINT: &str = "So11111111111111111111111111111111111111112";

pub struct MockedPriceOracle;

impl MockedPriceOracle {
    fn price_for(mint_address: &str) -> TokenPrice {
        let price = match mint_address {
            USDC_DEVNET_MINT => DEFAULT_MOCKED_USDC_PRICE,
            WSOL_DEVNET_MINT => DEFAULT_MOCKED_WSOL_PRICE,
            _ => DEFAULT_MOCKED_PRICE,
        };
        TokenPrice { price, confidence: 1.0, source: PriceSource::Mock, block_id: None }
    }
}

#[async_trait]
impl PriceOracle for MockedPriceOracle {
    async fn get_price(
        &self,
        _client: &Client,
        mint_address: &str,
    ) -> Result<TokenPrice, KoraError> {
        Ok(Self::price_for(mint_address))
    }

    async fn get_prices(
        &self,
        _client: &Client,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        Ok(mint_addresses.iter().map(|mint| (mint.clone(), Self::price_for(mint))).collect())
    }
}

pub struct OracleUtil {}

impl OracleUtil {
    pub fn get_mock_oracle_price() -> Arc<dyn PriceOracle + Send + Sync> {
        Arc::new(MockedPriceOracle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_mock_oracle_prices() {
        let oracle = OracleUtil::get_mock_oracle_price();
        let client = Client::new();

        let usdc_price = oracle.get_price(&client, USDC_DEVNET_MINT).await.unwrap();
        assert_eq!(usdc_price.price, DEFAULT_MOCKED_USDC_PRICE);
        assert_eq!(usdc_price.confidence, 1.0);
        assert_eq!(usdc_price.source, PriceSource::Mock);

        let sol_price = oracle.get_price(&client, WSOL_DEVNET_MINT).await.unwrap();
        assert_eq!(sol_price.price, DEFAULT_MOCKED_WSOL_PRICE);
        assert_eq!(sol_price.confidence, 1.0);
        assert_eq!(sol_price.source, PriceSource::Mock);

        let unknown_price = oracle.get_price(&client, "unknown_token").await.unwrap();
        assert_eq!(unknown_price.price, DEFAULT_MOCKED_PRICE);
        assert_eq!(unknown_price.confidence, 1.0);
        assert_eq!(unknown_price.source, PriceSource::Mock);
    }
}
