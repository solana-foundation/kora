use crate::{config::Config, error::KoraError, oracle::PriceSource, token::token::TokenUtil};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PriceModel {
    Margin { margin: f64 },
    Fixed { amount: u64, token: String, strict: bool },
    Free,
}

impl Default for PriceModel {
    fn default() -> Self {
        Self::Margin { margin: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct PriceConfig {
    #[serde(flatten)]
    pub model: PriceModel,
}

impl PriceConfig {
    pub async fn get_required_lamports_with_fixed(
        &self,
        config: &Config,
        rpc_client: &RpcClient,
        price_source: PriceSource,
    ) -> Result<u64, KoraError> {
        if let PriceModel::Fixed { amount, token, .. } = &self.model {
            return TokenUtil::calculate_token_value_in_lamports(
                config,
                *amount,
                &Pubkey::from_str(token).map_err(|e| {
                    log::error!("Invalid Pubkey for price {e}");

                    KoraError::ConfigError
                })?,
                price_source,
                rpc_client,
            )
            .await;
        }

        Err(KoraError::ConfigError)
    }

    pub async fn get_required_lamports_with_margin(
        &self,
        min_transaction_fee: u64,
    ) -> Result<u64, KoraError> {
        if let PriceModel::Margin { margin } = &self.model {
            let multiplier = 1.0 + margin;
            let result = min_transaction_fee as f64 * multiplier;

            // Check for overflow/underflow before casting to u64
            if result > u64::MAX as f64 || result < 0.0 {
                log::error!(
                    "Margin calculation overflow: min_transaction_fee={}, margin={}, result={}",
                    min_transaction_fee,
                    margin,
                    result
                );
                return Err(KoraError::ValidationError("Margin calculation overflow".to_string()));
            }

            return Ok(result as u64);
        }

        Err(KoraError::ConfigError)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::tests::{
        common::create_mock_rpc_client_with_mint,
        config_mock::{mock_state::get_config, ConfigMockBuilder},
    };

    #[tokio::test]
    async fn test_margin_model_get_required_lamports() {
        // Test margin of 0.1 (10%)
        let price_config = PriceConfig { model: PriceModel::Margin { margin: 0.1 } };

        let min_transaction_fee = 5000u64; // 5000 lamports base fee
        let expected_lamports = (5000.0 * 1.1) as u64; // 5500 lamports

        let result =
            price_config.get_required_lamports_with_margin(min_transaction_fee).await.unwrap();

        assert_eq!(result, expected_lamports);
    }

    #[tokio::test]
    async fn test_margin_model_get_required_lamports_zero_margin() {
        // Test margin of 0.0 (no margin)
        let price_config = PriceConfig { model: PriceModel::Margin { margin: 0.0 } };

        let min_transaction_fee = 5000u64;

        let result =
            price_config.get_required_lamports_with_margin(min_transaction_fee).await.unwrap();

        assert_eq!(result, min_transaction_fee);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_with_oracle() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let rpc_client = create_mock_rpc_client_with_mint(6); // USDC has 6 decimals

        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 1_000_000, // 1 USDC (1,000,000 base units with 6 decimals)
                token: usdc_mint.to_string(),
                strict: false,
            },
        };

        // Use Mock price source which returns 0.0001 SOL per USDC
        let price_source = PriceSource::Mock;

        let result =
            price_config.get_required_lamports_with_fixed(&config, &rpc_client, price_source).await.unwrap();

        // Expected calculation:
        // 1,000,000 base units / 10^6 = 1.0 USDC
        // 1.0 USDC * 0.0001 SOL/USDC = 0.0001 SOL
        // 0.0001 SOL * 1,000,000,000 lamports/SOL = 100,000 lamports
        assert_eq!(result, 100000);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_with_custom_price() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let rpc_client = create_mock_rpc_client_with_mint(9); // 9 decimals token

        let custom_token = "So11111111111111111111111111111111111111112"; // SOL mint
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 500000000, // 0.5 tokens (500,000,000 base units with 9 decimals)
                token: custom_token.to_string(),
                strict: false,
            },
        };

        // Mock oracle returns 1.0 SOL price for SOL mint
        let price_source = PriceSource::Mock;

        let result =
            price_config.get_required_lamports_with_fixed(&config, &rpc_client, price_source).await.unwrap();

        // Expected calculation:
        // 500,000,000 base units / 10^9 = 0.5 tokens
        // 0.5 tokens * 1.0 SOL/token = 0.5 SOL
        // 0.5 SOL * 1,000,000,000 lamports/SOL = 500,000,000 lamports
        assert_eq!(result, 500000000);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_small_amount() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let rpc_client = create_mock_rpc_client_with_mint(6); // USDC has 6 decimals

        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 1000, // 0.001 USDC (1,000 base units with 6 decimals)
                token: usdc_mint.to_string(),
                strict: false,
            },
        };

        let price_source = PriceSource::Mock;

        let result =
            price_config.get_required_lamports_with_fixed(&config, &rpc_client, price_source).await.unwrap();

        // Expected calculation:
        // 1,000 base units / 10^6 = 0.001 USDC
        // 0.001 USDC * 0.0001 SOL/USDC = 0.0000001 SOL
        // 0.0000001 SOL * 1,000,000,000 lamports/SOL = 100 lamports (rounded down)
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_default_price_config() {
        // Test that default creates Margin with 0.0 margin
        let default_config = PriceConfig::default();

        match default_config.model {
            PriceModel::Margin { margin } => assert_eq!(margin, 0.0),
            _ => panic!("Default should be Margin with 0.0 margin"),
        }
    }
}
