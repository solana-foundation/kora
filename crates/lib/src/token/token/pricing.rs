use crate::{
    config::Config,
    error::KoraError,
    oracle::TokenPrice,
    sanitize_error,
    token::{
        spl_token_2022::{Token2022Extensions, Token2022Mint},
        spl_token_2022_util::{MintExtension, ParsedExtension},
    },
    transaction::{ParsedSPLInstructionData, TokenMovement},
    CacheUtil,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{clock::Clock, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, sysvar};
use spl_token_2022_interface::extension::ExtensionType;
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

/// Seconds past the current cluster time a signed transaction may still land in. Bounds how far
/// an InterestBearing mint's rate can move its UI amount before the payment is processed.
const LANDING_WINDOW_SECS: i64 = 120;

/// Range of the factor a Token-2022 ScaledUiAmount or InterestBearing mint applies between raw
/// and UI amounts, across the time the transaction can land in. Value Kora receives at `lower`
/// and value Kora gives away at `upper`. The range always includes 1, the unscaled valuation,
/// because price sources differ on whether they quote per UI token or per raw token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct UiAmountScale {
    pub lower: Decimal,
    pub upper: Decimal,
}

impl UiAmountScale {
    pub const UNIT: Self = Self { lower: Decimal::ONE, upper: Decimal::ONE };

    fn from_factors(factors: &[f64]) -> Result<Self, KoraError> {
        let factors = factors
            .iter()
            .map(|factor| {
                Decimal::from_f64_retain(*factor).filter(|d| d.is_sign_positive() && !d.is_zero())
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                KoraError::ValidationError("Mint UI amount multiplier is not positive".to_string())
            })?;
        let lower = factors.iter().copied().fold(Decimal::ONE, Decimal::min);
        let upper = factors.iter().copied().fold(Decimal::ONE, Decimal::max);
        Ok(Self { lower, upper })
    }

    pub(super) fn for_mint(mint: &Token2022Mint, unix_timestamp: i64) -> Result<Self, KoraError> {
        if let Some(ParsedExtension::Mint(MintExtension::ScaledUiAmountConfig(config))) =
            mint.get_extension(ExtensionType::ScaledUiAmount)
        {
            let effective_at: i64 = config.new_multiplier_effective_timestamp.into();
            let current: f64 = if unix_timestamp >= effective_at {
                config.new_multiplier.into()
            } else {
                config.multiplier.into()
            };
            return Self::from_factors(&[current, config.new_multiplier.into()]);
        }

        if let Some(ParsedExtension::Mint(MintExtension::InterestBearingConfig(config))) =
            mint.get_extension(ExtensionType::InterestBearingConfig)
        {
            let unscaled_ui_amount = decimal_scale(mint.decimals)?
                .to_f64()
                .ok_or_else(|| KoraError::ValidationError("Invalid decimals scale".to_string()))?;
            let probe = 10u64.pow(MAX_SUPPORTED_DECIMALS);
            let factor_at = |timestamp: i64| -> Result<f64, KoraError> {
                config
                    .amount_to_ui_amount(probe, mint.decimals, timestamp)
                    .and_then(|ui| ui.parse::<f64>().ok())
                    .map(|ui| ui * unscaled_ui_amount / probe as f64)
                    .ok_or_else(|| {
                        KoraError::ValidationError(
                            "Failed to compute interest-bearing UI amount".to_string(),
                        )
                    })
            };
            return Self::from_factors(&[
                factor_at(unix_timestamp)?,
                factor_at(unix_timestamp.saturating_add(LANDING_WINDOW_SECS))?,
            ]);
        }

        Ok(Self::UNIT)
    }
}

impl TokenUtil {
    async fn get_mint_decimals_and_scale(
        config: &Config,
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<(u8, UiAmountScale), KoraError> {
        let mint = Self::get_mint(config, rpc_client, mint_pubkey).await?;
        let decimals = mint.decimals();
        let Some(mint_2022) = mint.as_any().downcast_ref::<Token2022Mint>() else {
            return Ok((decimals, UiAmountScale::UNIT));
        };
        let extension_types = mint_2022.get_extension_types();
        if !extension_types.contains(&ExtensionType::ScaledUiAmount)
            && !extension_types.contains(&ExtensionType::InterestBearingConfig)
        {
            return Ok((decimals, UiAmountScale::UNIT));
        }

        let clock_account = rpc_client.get_account(&sysvar::clock::id()).await?;
        let clock: Clock = bincode::deserialize(&clock_account.data)?;
        Ok((decimals, UiAmountScale::for_mint(mint_2022, clock.unix_timestamp)?))
    }

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
                            KoraError::RpcError(format!(
                                "Failed to get current slot: {}",
                                sanitize_error!(e)
                            ))
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
    ) -> Result<(HashMap<String, TokenPrice>, HashMap<Pubkey, (u8, UiAmountScale)>), KoraError>
    {
        let mint_addresses: Vec<String> = mints.iter().map(|mint| mint.to_string()).collect();

        let prices =
            CacheUtil::get_or_fetch_token_prices(rpc_client, config, &mint_addresses).await?;

        let current_slot = if config.validation.max_price_staleness_slots > 0 {
            Some(rpc_client.get_slot().await.map_err(|e| {
                KoraError::RpcError(format!("Failed to get current slot: {}", sanitize_error!(e)))
            })?)
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
            mint_decimals
                .insert(*mint, Self::get_mint_decimals_and_scale(config, rpc_client, mint).await?);
        }

        Ok((prices, mint_decimals))
    }

    pub async fn get_token_price_and_decimals(
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<(TokenPrice, u8), KoraError> {
        let (token_price, decimals, _) =
            Self::get_token_price_decimals_and_scale(mint, rpc_client, config).await?;
        Ok((token_price, decimals))
    }

    async fn get_token_price_decimals_and_scale(
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<(TokenPrice, u8, UiAmountScale), KoraError> {
        let (decimals, scale) = Self::get_mint_decimals_and_scale(config, rpc_client, mint).await?;

        // Get token price in SOL directly (cached when Redis is enabled).
        let token_price =
            CacheUtil::get_or_fetch_token_price(rpc_client, config, &mint.to_string())
                .await
                .map_err(|e| {
                    KoraError::RpcError(format!(
                        "Failed to fetch token price: {}",
                        sanitize_error!(e)
                    ))
                })?;

        Self::check_price_staleness(rpc_client, config, &token_price, "", None).await?;

        Ok((token_price, decimals, scale))
    }

    pub(super) fn scaled_price(price: Decimal, factor: Decimal) -> Result<Decimal, KoraError> {
        price.checked_mul(factor).ok_or_else(|| {
            KoraError::ValidationError("Scaled token price calculation overflow".to_string())
        })
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
        let (token_price, decimals, ui_scale) =
            Self::get_token_price_decimals_and_scale(mint, rpc_client, config).await?;
        Self::calculate_token_value_in_lamports_from_price(
            amount,
            Self::scaled_price(token_price.price, ui_scale.lower)?,
            decimals,
        )
    }

    pub async fn calculate_lamports_value_in_token(
        lamports: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals, ui_scale) =
            Self::get_token_price_decimals_and_scale(mint, rpc_client, config).await?;
        let price = Self::scaled_price(token_price.price, ui_scale.lower)?;

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
            .and_then(|result| result.checked_div(lamports_per_sol_decimal.checked_mul(price)?))
            .ok_or_else(|| {
                log::error!("Token value calculation overflow: lamports={}, scale={}, lamports_per_sol_decimal={}, price={}",
                    lamports,
                    scale,
                    lamports_per_sol_decimal,
                    price
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

    /// Calculate the total lamports value of SPL tokens the fee payer transfers out, burns, or
    /// approves a delegate to spend.
    pub async fn calculate_spl_transfers_value_in_lamports(
        spl_transfers: &[ParsedSPLInstructionData],
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        // Collect all outflow transfers (fee payer as source) grouped by mint
        let mut mint_to_transfers: HashMap<Pubkey, Vec<u64>> = HashMap::new();

        for transfer in spl_transfers {
            if let Some(TokenMovement {
                amount,
                owner,
                multisig_signers,
                mint,
                source_address,
                ..
            }) = transfer.token_movement()
            {
                if owner == *fee_payer || multisig_signers.contains(fee_payer) {
                    let mint_pubkey = if let Some(m) = mint {
                        m
                    } else {
                        let source_account =
                            CacheUtil::get_account(config, rpc_client, &source_address, false)
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
                    if !config.validation.allowed_tokens.contains(&mint_pubkey.to_string()) {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Fee payer cannot move tokens of mint {mint_pubkey}: it is not in allowed_tokens"
                        )));
                    }
                    mint_to_transfers.entry(mint_pubkey).or_default().push(amount);
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
            let (decimals, ui_scale) = mint_decimals
                .get(mint)
                .ok_or_else(|| KoraError::RpcError(format!("No decimals data for mint {mint}")))?;
            let outflow_price = Self::scaled_price(price.price, ui_scale.upper)?;

            for amount in transfers {
                let lamports = Self::calculate_token_value_in_lamports_from_price(
                    *amount,
                    outflow_price,
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

#[cfg(test)]
mod tests {
    use super::*;
    use spl_token_2022_interface::extension::{
        interest_bearing_mint::InterestBearingConfig, scaled_ui_amount::ScaledUiAmountConfig,
    };

    fn mint_with(extension_type: ExtensionType, extension: MintExtension) -> Token2022Mint {
        Token2022Mint {
            mint: Pubkey::new_unique(),
            mint_authority: None,
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
            extensions_types: vec![extension_type],
            extensions: HashMap::from([(extension_type as u16, ParsedExtension::Mint(extension))]),
        }
    }

    fn scaled_ui_mint(multiplier: f64, effective_at: i64, new_multiplier: f64) -> Token2022Mint {
        mint_with(
            ExtensionType::ScaledUiAmount,
            MintExtension::ScaledUiAmountConfig(ScaledUiAmountConfig {
                multiplier: multiplier.into(),
                new_multiplier_effective_timestamp: effective_at.into(),
                new_multiplier: new_multiplier.into(),
                ..Default::default()
            }),
        )
    }

    #[test]
    fn test_ui_amount_scale_spans_current_and_pending_multiplier() {
        let scale = UiAmountScale::for_mint(&scaled_ui_mint(0.5, 1_000, 2.0), 999).unwrap();
        assert_eq!(scale.lower, Decimal::from_f64_retain(0.5).unwrap());
        assert_eq!(scale.upper, Decimal::from_f64_retain(2.0).unwrap());
    }

    #[test]
    fn test_ui_amount_scale_uses_new_multiplier_once_effective() {
        let scale = UiAmountScale::for_mint(&scaled_ui_mint(0.5, 1_000, 2.0), 1_000).unwrap();
        assert_eq!(scale.lower, Decimal::ONE);
        assert_eq!(scale.upper, Decimal::from_f64_retain(2.0).unwrap());
    }

    #[test]
    fn test_ui_amount_scale_never_credits_above_or_charges_below_the_raw_amount() {
        let above = UiAmountScale::for_mint(&scaled_ui_mint(3.0, i64::MAX, 3.0), 0).unwrap();
        assert_eq!(above.lower, Decimal::ONE, "a payment is not worth more than its raw amount");
        assert_eq!(above.upper, Decimal::from_f64_retain(3.0).unwrap());

        let below = UiAmountScale::for_mint(&scaled_ui_mint(0.25, i64::MAX, 0.25), 0).unwrap();
        assert_eq!(below.lower, Decimal::from_f64_retain(0.25).unwrap());
        assert_eq!(below.upper, Decimal::ONE, "an outflow is not worth less than its raw amount");
    }

    #[test]
    fn test_ui_amount_scale_rejects_non_positive_multiplier() {
        assert!(UiAmountScale::for_mint(&scaled_ui_mint(0.0, 0, 0.0), 1).is_err());
    }

    #[test]
    fn test_ui_amount_scale_tracks_interest_accrued_by_landing() {
        let one_year = 31_556_736;
        let mint = mint_with(
            ExtensionType::InterestBearingConfig,
            MintExtension::InterestBearingConfig(InterestBearingConfig {
                initialization_timestamp: 0.into(),
                last_update_timestamp: 0.into(),
                pre_update_average_rate: 10_000.into(),
                current_rate: 10_000.into(),
                ..Default::default()
            }),
        );

        let scale = UiAmountScale::for_mint(&mint, one_year).unwrap();

        let e = Decimal::from_f64_retain(std::f64::consts::E).unwrap();
        assert_eq!(scale.lower, Decimal::ONE);
        assert!(
            scale.upper > e,
            "interest keeps accruing until the payment lands: {}",
            scale.upper
        );
        assert!((scale.upper - e).abs() < Decimal::new(1, 3), "upper {}", scale.upper);
    }

    #[test]
    fn test_ui_amount_scale_is_unit_without_scaling_extension() {
        let mint = mint_with(
            ExtensionType::MintCloseAuthority,
            MintExtension::MintCloseAuthority(Default::default()),
        );
        assert_eq!(UiAmountScale::for_mint(&mint, 0).unwrap(), UiAmountScale::UNIT);
    }

    #[test]
    fn test_scaled_price_halves_value_of_half_multiplier_mint() {
        let price = Decimal::new(1, 3);
        let scale = UiAmountScale::for_mint(&scaled_ui_mint(0.5, i64::MAX, 0.5), 0).unwrap();

        let unscaled =
            TokenUtil::calculate_token_value_in_lamports_from_price(1_000_000, price, 6).unwrap();
        let scaled = TokenUtil::calculate_token_value_in_lamports_from_price(
            1_000_000,
            TokenUtil::scaled_price(price, scale.lower).unwrap(),
            6,
        )
        .unwrap();

        assert_eq!(unscaled, 1_000_000);
        assert_eq!(scaled, 500_000);
    }
}
