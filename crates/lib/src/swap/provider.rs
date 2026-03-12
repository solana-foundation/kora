use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

use crate::{
    config::{Config, SwapQuoteProviderType},
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle},
    token::token::TokenUtil,
};

#[async_trait]
pub trait SwapQuoteProvider: Send + Sync {
    async fn quote_token_amount_in_for_lamports_out(
        &self,
        rpc_client: &RpcClient,
        token_mint: &Pubkey,
        lamports_out: u64,
        config: &Config,
    ) -> Result<u64, KoraError>;
}

pub fn get_swap_quote_provider(config: &Config) -> Arc<dyn SwapQuoteProvider + Send + Sync> {
    match config.kora.swap_for_gas.quote_provider {
        SwapQuoteProviderType::Jupiter => {
            Arc::new(OracleBackedSwapQuoteProvider { price_source: PriceSource::Jupiter })
        }
        SwapQuoteProviderType::Mock => {
            Arc::new(OracleBackedSwapQuoteProvider { price_source: PriceSource::Mock })
        }
    }
}

struct OracleBackedSwapQuoteProvider {
    price_source: PriceSource,
}

impl OracleBackedSwapQuoteProvider {
    async fn get_price_in_sol(
        &self,
        rpc_client: &RpcClient,
        token_mint: &Pubkey,
        config: &Config,
    ) -> Result<Decimal, KoraError> {
        let oracle = RetryingPriceOracle::new(
            3,
            Duration::from_secs(1),
            get_price_oracle(self.price_source.clone())?,
        );

        let token_price = oracle.get_token_price(&token_mint.to_string()).await?;

        if matches!(self.price_source, PriceSource::Jupiter) {
            let max_staleness = config.validation.max_price_staleness_slots;
            if max_staleness > 0 {
                let block_id = token_price.block_id.ok_or_else(|| {
                    KoraError::ValidationError(
                        "Oracle price data has no block_id; cannot verify staleness".to_string(),
                    )
                })?;
                let current_slot = rpc_client
                    .get_slot()
                    .await
                    .map_err(|e| KoraError::RpcError(format!("Failed to get current slot: {e}")))?;
                let age = current_slot.saturating_sub(block_id);
                if age > max_staleness {
                    return Err(KoraError::ValidationError(format!(
                        "Oracle price data is stale: age {age} slots exceeds max {max_staleness} slots"
                    )));
                }
            }
        }

        Ok(token_price.price)
    }

    fn calculate_token_amount_in(
        &self,
        lamports_out: u64,
        token_price_in_sol: Decimal,
        token_decimals: u8,
    ) -> Result<u64, KoraError> {
        let lamports_decimal = Decimal::from_u64(lamports_out)
            .ok_or_else(|| KoraError::ValidationError("Invalid lamports value".to_string()))?;
        let lamports_per_sol = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;
        let token_scale = Decimal::from_u64(10u64.pow(token_decimals as u32))
            .ok_or_else(|| KoraError::ValidationError("Invalid token decimals".to_string()))?;

        let token_amount = lamports_decimal
            .checked_mul(token_scale)
            .and_then(|r| r.checked_div(lamports_per_sol.checked_mul(token_price_in_sol)?))
            .ok_or_else(|| {
                KoraError::ValidationError("Token quote calculation overflow".to_string())
            })?;

        token_amount
            .ceil()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Token quote amount overflow".to_string()))
    }
}

#[async_trait]
impl SwapQuoteProvider for OracleBackedSwapQuoteProvider {
    async fn quote_token_amount_in_for_lamports_out(
        &self,
        rpc_client: &RpcClient,
        token_mint: &Pubkey,
        lamports_out: u64,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let token_price_in_sol = self.get_price_in_sol(rpc_client, token_mint, config).await?;
        let token_decimals = TokenUtil::get_mint_decimals(config, rpc_client, token_mint).await?;

        self.calculate_token_amount_in(lamports_out, token_price_in_sol, token_decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_token_amount_in_rounds_up() {
        let provider = OracleBackedSwapQuoteProvider { price_source: PriceSource::Mock };

        let amount = provider
            .calculate_token_amount_in(1, Decimal::from(1), 6)
            .expect("quote calculation should succeed");

        assert_eq!(amount, 1);
    }
}
