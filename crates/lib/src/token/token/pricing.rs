use crate::{
    config::Config, error::KoraError, oracle::TokenPrice, transaction::ParsedSPLInstructionData,
    CacheUtil,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::collections::HashMap;

use super::{TokenType, TokenUtil};

const MAX_SUPPORTED_DECIMALS: u32 = 19;

pub(super) fn decimal_scale(decimals: u8) -> Result<Decimal, KoraError> {
    if (decimals as u32) > MAX_SUPPORTED_DECIMALS {
        return Err(KoraError::ValidationError(format!(
            "Token decimals {} exceeds maximum supported value of {}",
            decimals, MAX_SUPPORTED_DECIMALS
        )));
    }
    Decimal::from_u64(10u64.pow(decimals as u32))
        .ok_or_else(|| KoraError::ValidationError("Invalid decimals scale".to_string()))
}

impl TokenUtil {
    async fn check_price_staleness(
        rpc_client: &RpcClient,
        config: &Config,
        price: &TokenPrice,
        label: &str,
        current_slot: Option<u64>,
    ) -> Result<(), KoraError> {
        let max_staleness = config.validation.max_price_staleness_slots;
        if max_staleness > 0 {
            match price.block_id {
                Some(block_id) => {
                    let slot = match current_slot {
                        Some(s) => s,
                        None => rpc_client.get_slot().await.map_err(|e| {
                            KoraError::RpcError(format!("Failed to get current slot: {e}"))
                        })?,
                    };
                    let age = slot.saturating_sub(block_id);
                    if age > max_staleness {
                        return Err(KoraError::ValidationError(format!(
                            "Oracle price data{} is stale: age {} slots exceeds max {} slots",
                            label, age, max_staleness
                        )));
                    }
                }
                None => {
                    return Err(KoraError::ValidationError(format!(
                        "Oracle price data{} has no block_id; cannot verify staleness",
                        label
                    )));
                }
            }
        }
        Ok(())
    }

    pub(super) async fn fetch_prices_and_decimals(
        config: &Config,
        rpc_client: &RpcClient,
        mints: &[Pubkey],
    ) -> Result<(HashMap<String, TokenPrice>, HashMap<Pubkey, u8>), KoraError> {
        let mint_addresses: Vec<String> = mints.iter().map(|mint| mint.to_string()).collect();

        let prices =
            CacheUtil::get_or_fetch_token_prices(rpc_client, config, &mint_addresses).await?;

        let current_slot = if config.validation.max_price_staleness_slots > 0 {
            Some(
                rpc_client
                    .get_slot()
                    .await
                    .map_err(|e| KoraError::RpcError(format!("Failed to get current slot: {e}")))?,
            )
        } else {
            None
        };

        for (mint_addr, price) in &prices {
            Self::check_price_staleness(
                rpc_client,
                config,
                price,
                &format!(" for {mint_addr}"),
                current_slot,
            )
            .await?;
        }

        let mut mint_decimals = HashMap::new();
        for mint in mints {
            let decimals = Self::get_mint_decimals(config, rpc_client, mint).await?;
            mint_decimals.insert(*mint, decimals);
        }

        Ok((prices, mint_decimals))
    }

    pub async fn get_token_price_and_decimals(
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<(TokenPrice, u8), KoraError> {
        let decimals = Self::get_mint_decimals(config, rpc_client, mint).await?;

        // Get token price in SOL directly (cached when Redis is enabled).
        let token_price =
            CacheUtil::get_or_fetch_token_price(rpc_client, config, &mint.to_string())
                .await
                .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {e}")))?;

        Self::check_price_staleness(rpc_client, config, &token_price, "", None).await?;

        Ok((token_price, decimals))
    }

    pub(super) fn calculate_token_value_in_lamports_from_price(
        amount: u64,
        price: Decimal,
        decimals: u8,
    ) -> Result<u64, KoraError> {
        let amount_decimal = Decimal::from_u64(amount)
            .ok_or_else(|| KoraError::ValidationError("Invalid token amount".to_string()))?;
        let decimals_scale = decimal_scale(decimals)?;
        let lamports_per_sol = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;

        let lamports_decimal = amount_decimal.checked_mul(price)
            .and_then(|result| result.checked_mul(lamports_per_sol))
            .and_then(|result| result.checked_div(decimals_scale))
            .ok_or_else(|| {
                log::error!("Token value calculation overflow: amount={}, price={}, decimals={}, lamports_per_sol={}",
                    amount,
                    price,
                    decimals,
                    lamports_per_sol
                );
                KoraError::ValidationError("Token value calculation overflow".to_string())
            })?;

        lamports_decimal
            .floor()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Lamports value overflow".to_string()))
    }

    pub async fn calculate_token_value_in_lamports(
        amount: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, rpc_client, config).await?;
        Self::calculate_token_value_in_lamports_from_price(amount, token_price.price, decimals)
    }

    pub async fn calculate_lamports_value_in_token(
        lamports: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, rpc_client, config).await?;

        // Convert lamports to token base units
        let lamports_decimal = Decimal::from_u64(lamports)
            .ok_or_else(|| KoraError::ValidationError("Invalid lamports value".to_string()))?;
        let lamports_per_sol_decimal = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;
        let scale = decimal_scale(decimals)?;

        // Calculate: (lamports * 10^decimals) / (LAMPORTS_PER_SOL * price)
        // Multiply before divide to preserve precision
        let token_amount = lamports_decimal
            .checked_mul(scale)
            .and_then(|result| result.checked_div(lamports_per_sol_decimal.checked_mul(token_price.price)?))
            .ok_or_else(|| {
                log::error!("Token value calculation overflow: lamports={}, scale={}, lamports_per_sol_decimal={}, token_price.price={}",
                    lamports,
                    scale,
                    lamports_per_sol_decimal,
                    token_price.price
                );
                KoraError::ValidationError("Token value calculation overflow".to_string())
            })?;

        // Ceil and convert to u64
        let result = token_amount
            .ceil()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Token amount overflow".to_string()))?;

        Ok(result)
    }

    /// Calculate the total lamports value of SPL token transfers where the fee payer is involved
    /// This includes both outflow (fee payer as owner/source) and inflow (fee payer owns destination)
    pub async fn calculate_spl_transfers_value_in_lamports(
        spl_transfers: &[ParsedSPLInstructionData],
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        // Collect all outflow transfers (fee payer as source) grouped by mint
        let mut mint_to_transfers: HashMap<Pubkey, Vec<u64>> = HashMap::new();

        for transfer in spl_transfers {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner,
                mint,
                source_address,
                ..
            } = transfer
            {
                // Only count outflows (fee payer as source)
                if *owner == *fee_payer {
                    let mint_pubkey = if let Some(m) = mint {
                        *m
                    } else {
                        let source_account =
                            CacheUtil::get_account(config, rpc_client, source_address, false)
                                .await?;
                        let token_program =
                            TokenType::get_token_program_from_owner(&source_account.owner)?;
                        let token_account = token_program
                            .unpack_token_account(&source_account.data)
                            .map_err(|e| {
                                KoraError::TokenOperationError(format!(
                                    "Failed to unpack source token account {}: {}",
                                    source_address, e
                                ))
                            })?;
                        token_account.mint()
                    };
                    mint_to_transfers.entry(mint_pubkey).or_default().push(*amount);
                }
            }
        }

        if mint_to_transfers.is_empty() {
            return Ok(0);
        }

        let mints: Vec<Pubkey> = mint_to_transfers.keys().copied().collect();
        let (prices, mint_decimals) =
            Self::fetch_prices_and_decimals(config, rpc_client, &mints).await?;

        let mut total_lamports: u64 = 0;

        for (mint, transfers) in mint_to_transfers.iter() {
            let price = prices
                .get(&mint.to_string())
                .ok_or_else(|| KoraError::RpcError(format!("No price data for mint {mint}")))?;
            let decimals = mint_decimals
                .get(mint)
                .ok_or_else(|| KoraError::RpcError(format!("No decimals data for mint {mint}")))?;

            for amount in transfers {
                let lamports = Self::calculate_token_value_in_lamports_from_price(
                    *amount,
                    price.price,
                    *decimals,
                )?;

                total_lamports = total_lamports.checked_add(lamports).ok_or_else(|| {
                    log::error!("SPL outflow calculation overflow");
                    KoraError::ValidationError("SPL outflow calculation overflow".to_string())
                })?;
            }
        }

        Ok(total_lamports)
    }
}
