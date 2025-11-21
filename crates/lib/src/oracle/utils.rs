use crate::oracle::{MockPriceOracle, PriceOracle, PriceSource, TokenPrice};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_MOCKED_PRICE: Decimal = dec!(0.001);
pub const DEFAULT_MOCKED_USDC_PRICE: Decimal = dec!(0.0001);
pub const DEFAULT_MOCKED_WSOL_PRICE: Decimal = dec!(1.0);

pub const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
pub const WSOL_DEVNET_MINT: &str = "So11111111111111111111111111111111111111112";

pub struct OracleUtil {}

impl OracleUtil {
    pub fn get_mock_oracle_price() -> Arc<dyn PriceOracle + Send + Sync> {
        let mut mock = MockPriceOracle::new();
        // Set up default mock behavior for devnet tokens
        mock.expect_get_price()
            .times(..) // Allow unlimited calls
            .returning(|_, mint_address| {
                let price = match mint_address {
                    USDC_DEVNET_MINT => DEFAULT_MOCKED_USDC_PRICE, // USDC
                    WSOL_DEVNET_MINT => DEFAULT_MOCKED_WSOL_PRICE, // SOL
                    _ => DEFAULT_MOCKED_PRICE, // Default price for unknown tokens
                };
                Ok(TokenPrice { price, confidence: 1.0, source: PriceSource::Mock })
            });

        mock.expect_get_prices()
            .times(..) // Allow unlimited calls
            .returning(|_, mint_addresses| {
                let mut result = HashMap::new();
                for mint_address in mint_addresses {
                    let price = match mint_address.as_str() {
                        USDC_DEVNET_MINT => DEFAULT_MOCKED_USDC_PRICE, // USDC
                        WSOL_DEVNET_MINT => DEFAULT_MOCKED_WSOL_PRICE, // SOL
                        _ => DEFAULT_MOCKED_PRICE, // Default price for unknown tokens
                    };
                    result.insert(
                        mint_address.clone(),
                        TokenPrice { price, confidence: 1.0, source: PriceSource::Mock },
                    );
                }
                Ok(result)
            });
        Arc::new(mock)
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

        // Test USDC price
        let usdc_price = oracle.get_price(&client, USDC_DEVNET_MINT).await.unwrap();
        assert_eq!(usdc_price.price, DEFAULT_MOCKED_USDC_PRICE);
        assert_eq!(usdc_price.confidence, 1.0);
        assert_eq!(usdc_price.source, PriceSource::Mock);

        // Test SOL price
        let sol_price = oracle.get_price(&client, WSOL_DEVNET_MINT).await.unwrap();
        assert_eq!(sol_price.price, DEFAULT_MOCKED_WSOL_PRICE);
        assert_eq!(sol_price.confidence, 1.0);
        assert_eq!(sol_price.source, PriceSource::Mock);

        // Test unknown token (should return default price)
        let unknown_price = oracle.get_price(&client, "unknown_token").await.unwrap();
        assert_eq!(unknown_price.price, DEFAULT_MOCKED_PRICE);
        assert_eq!(unknown_price.confidence, 1.0);
        assert_eq!(unknown_price.source, PriceSource::Mock);
    }
}
