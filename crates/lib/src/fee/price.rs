use crate::{config::Config, error::KoraError, token::token::TokenUtil};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PriceModel {
    Margin {
        margin: f64,
    },
    Fixed {
        amount: u64,
        token: String,
        strict: bool,
    },
    /// Charge a fixed fee payable in any of the listed stablecoin mints.
    /// `amount` is in base units of the first listed mint, which anchors the fee; a payer
    /// using any other listed mint is charged the equivalent value at current oracle prices.
    /// Use this when you accept several stablecoins (USDC, USDT, PYUSD, …) for one fee.
    FixedStable {
        amount: u64,
        tokens: Vec<String>,
        strict: bool,
    },
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
    async fn token_amount_to_lamports(
        amount: u64,
        token: &str,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let mint = Pubkey::from_str(token).map_err(|e| {
            log::error!("Invalid Pubkey for price {e}");
            KoraError::ConfigError(
                "Invalid token address in fee config: failed to parse as Solana pubkey".to_string(),
            )
        })?;
        TokenUtil::calculate_token_value_in_lamports(amount, &mint, rpc_client, config).await
    }

    pub async fn get_required_lamports_with_fixed(
        &self,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        if let PriceModel::Fixed { amount, token, .. } = &self.model {
            return Self::token_amount_to_lamports(*amount, token, rpc_client, config).await;
        }

        Err(KoraError::ConfigError(
            "Price model is not 'Fixed': cannot compute fixed fee".to_string(),
        ))
    }

    /// Lamport equivalent of the fixed fee, anchored to the first listed mint.
    /// This is the reference value used for `max_allowed_lamports` and strict-pricing
    /// enforcement; the actual token a payer uses is converted at oracle rates in
    /// [`FeeConfigUtil::calculate_fee_in_token`].
    pub async fn get_required_lamports_with_fixed_stable(
        &self,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        if let PriceModel::FixedStable { amount, tokens, .. } = &self.model {
            let reference_token = tokens.first().ok_or_else(|| {
                KoraError::ConfigError(
                    "FixedStable price model requires at least one token".to_string(),
                )
            })?;
            return Self::token_amount_to_lamports(*amount, reference_token, rpc_client, config)
                .await;
        }

        Err(KoraError::ConfigError(
            "Price model is not 'FixedStable': cannot compute fixed stable fee".to_string(),
        ))
    }

    pub async fn get_required_lamports_with_margin(
        &self,
        min_transaction_fee: u64,
    ) -> Result<u64, KoraError> {
        if let PriceModel::Margin { margin } = &self.model {
            let margin_decimal = Decimal::from_f64(*margin)
                .ok_or_else(|| KoraError::ValidationError("Invalid margin".to_string()))?;

            let multiplier = Decimal::from_u64(1u64)
                .and_then(|result| result.checked_add(margin_decimal))
                .ok_or_else(|| {
                    log::error!(
                        "Multiplier calculation overflow: min_transaction_fee={}, margin={}",
                        min_transaction_fee,
                        margin,
                    );
                    KoraError::ValidationError("Multiplier calculation overflow".to_string())
                })?;

            let result = Decimal::from_u64(min_transaction_fee)
                .and_then(|result| result.checked_mul(multiplier))
                .ok_or_else(|| {
                    log::error!(
                        "Margin calculation overflow: min_transaction_fee={}, margin={}",
                        min_transaction_fee,
                        margin,
                    );
                    KoraError::ValidationError("Margin calculation overflow".to_string())
                })?;

            return result.ceil().to_u64().ok_or_else(|| {
                log::error!(
                    "Margin calculation overflow: min_transaction_fee={}, margin={}, result={}",
                    min_transaction_fee,
                    margin,
                    result
                );
                KoraError::ValidationError("Margin calculation overflow".to_string())
            });
        }

        Err(KoraError::ConfigError(
            "Price model is not 'Margin': cannot compute margin fee".to_string(),
        ))
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

        // Use Mock price source which returns 0.0075 SOL per USDC

        let result =
            price_config.get_required_lamports_with_fixed(&rpc_client, &config).await.unwrap();

        // Expected calculation:
        // 1,000,000 base units / 10^6 = 1.0 USDC
        // 1.0 USDC * 0.0075 SOL/USDC = 0.0075 SOL
        // 0.0075 SOL * 1,000,000,000 lamports/SOL = 7,500,000 lamports
        assert_eq!(result, 7500000);
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

        let result =
            price_config.get_required_lamports_with_fixed(&rpc_client, &config).await.unwrap();

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

        let result =
            price_config.get_required_lamports_with_fixed(&rpc_client, &config).await.unwrap();

        // Expected calculation:
        // 1,000 base units / 10^6 = 0.001 USDC
        // 0.001 USDC * 0.0075 SOL/USDC = 0.0000075 SOL
        // 0.0000075 SOL * 1,000,000,000 lamports/SOL = 7,500 lamports
        assert_eq!(result, 7500);
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

    #[tokio::test]
    async fn test_fixed_stable_model_get_required_lamports_uses_first_token() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let rpc_client = create_mock_rpc_client_with_mint(6);

        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        let other_stable = "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3"; // some other stable mint
        let price_config = PriceConfig {
            model: PriceModel::FixedStable {
                amount: 1_000_000, // 1 USDC
                tokens: vec![usdc_mint.to_string(), other_stable.to_string()],
                strict: false,
            },
        };

        let result = price_config
            .get_required_lamports_with_fixed_stable(&rpc_client, &config)
            .await
            .unwrap();

        // First token (USDC) is used as reference: 1 USDC * 0.0075 SOL/USDC * 1e9 = 7,500,000 lamports
        assert_eq!(result, 7_500_000);
    }

    #[tokio::test]
    async fn test_fixed_stable_model_empty_tokens_returns_error() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let rpc_client = create_mock_rpc_client_with_mint(6);

        let price_config = PriceConfig {
            model: PriceModel::FixedStable { amount: 1_000_000, tokens: vec![], strict: false },
        };

        let result =
            price_config.get_required_lamports_with_fixed_stable(&rpc_client, &config).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("at least one token"),
            "Expected 'at least one token' in error: {err}"
        );
    }
}
