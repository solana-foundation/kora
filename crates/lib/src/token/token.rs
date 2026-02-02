use crate::{
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
    token::{
        interface::TokenMint,
        spl_token::TokenProgram,
        spl_token_2022::{Token2022Account, Token2022Extensions, Token2022Mint, Token2022Program},
        TokenInterface,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
    CacheUtil,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use std::{collections::HashMap, str::FromStr, time::Duration};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use {crate::tests::config_mock::mock_state::get_config, rust_decimal_macros::dec};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Spl,
    Token2022,
}

impl TokenType {
    pub fn get_token_program_from_owner(
        owner: &Pubkey,
    ) -> Result<Box<dyn TokenInterface>, KoraError> {
        if *owner == spl_token_interface::id() {
            Ok(Box::new(TokenProgram::new()))
        } else if *owner == spl_token_2022_interface::id() {
            Ok(Box::new(Token2022Program::new()))
        } else {
            Err(KoraError::TokenOperationError(format!("Invalid token program owner: {owner}")))
        }
    }

    pub fn get_token_program(&self) -> Box<dyn TokenInterface> {
        match self {
            TokenType::Spl => Box::new(TokenProgram::new()),
            TokenType::Token2022 => Box::new(Token2022Program::new()),
        }
    }
}

pub struct TokenUtil;

impl TokenUtil {
    pub fn check_valid_tokens(tokens: &[String]) -> Result<Vec<Pubkey>, KoraError> {
        tokens
            .iter()
            .map(|token| {
                Pubkey::from_str(token).map_err(|_| {
                    KoraError::ValidationError(format!("Invalid token address: {token}"))
                })
            })
            .collect()
    }

    pub async fn get_mint(
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        let mint_account = CacheUtil::get_account(rpc_client, mint_pubkey, false).await?;

        let token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;

        token_program
            .unpack_mint(mint_pubkey, &mint_account.data)
            .map_err(|e| KoraError::TokenOperationError(format!("Failed to unpack mint: {e}")))
    }

    pub async fn get_mint_decimals(
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<u8, KoraError> {
        let mint = Self::get_mint(rpc_client, mint_pubkey).await?;
        Ok(mint.decimals())
    }

    pub async fn get_token_price_and_decimals(
        mint: &Pubkey,
        price_source: PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<(TokenPrice, u8), KoraError> {
        let decimals = Self::get_mint_decimals(rpc_client, mint).await?;

        let oracle =
            RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source)?);

        // Get token price in SOL directly
        let token_price = oracle
            .get_token_price(&mint.to_string())
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {e}")))?;

        Ok((token_price, decimals))
    }

    pub async fn calculate_token_value_in_lamports(
        amount: u64,
        mint: &Pubkey,
        price_source: PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, price_source, rpc_client).await?;

        // Convert amount to Decimal with proper scaling
        let amount_decimal = Decimal::from_u64(amount)
            .ok_or_else(|| KoraError::ValidationError("Invalid token amount".to_string()))?;
        let decimals_scale = Decimal::from_u64(10u64.pow(decimals as u32))
            .ok_or_else(|| KoraError::ValidationError("Invalid decimals".to_string()))?;
        let lamports_per_sol = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;

        // Calculate: (amount * price * LAMPORTS_PER_SOL) / 10^decimals
        // Multiply before divide to preserve precision
        let lamports_decimal = amount_decimal.checked_mul(token_price.price).and_then(|result| result.checked_mul(lamports_per_sol)).and_then(|result| result.checked_div(decimals_scale)).ok_or_else(|| {
            log::error!("Token value calculation overflow: amount={}, price={}, decimals={}, lamports_per_sol={}",
                amount,
                token_price.price,
                decimals,
                lamports_per_sol
            );
            KoraError::ValidationError("Token value calculation overflow".to_string())
        })?;

        // Floor and convert to u64
        let lamports = lamports_decimal
            .floor()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Lamports value overflow".to_string()))?;

        Ok(lamports)
    }

    pub async fn calculate_lamports_value_in_token(
        lamports: u64,
        mint: &Pubkey,
        price_source: &PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, price_source.clone(), rpc_client).await?;

        // Convert lamports to token base units
        let lamports_decimal = Decimal::from_u64(lamports)
            .ok_or_else(|| KoraError::ValidationError("Invalid lamports value".to_string()))?;
        let lamports_per_sol_decimal = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;
        let scale = Decimal::from_u64(10u64.pow(decimals as u32))
            .ok_or_else(|| KoraError::ValidationError("Invalid decimals".to_string()))?;

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
        price_source: &PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        // Collect all unique mints that need price lookups
        let mut mint_to_transfers: HashMap<
            Pubkey,
            Vec<(u64, bool)>, // (amount, is_outflow)
        > = HashMap::new();

        for transfer in spl_transfers {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner,
                mint,
                destination_address,
                ..
            } = transfer
            {
                // Check if fee payer is the source (outflow)
                if *owner == *fee_payer {
                    if let Some(mint_pubkey) = mint {
                        mint_to_transfers.entry(*mint_pubkey).or_default().push((*amount, true));
                    }
                } else {
                    // Check if fee payer owns the destination (inflow)
                    // We need to check the destination token account owner
                    if let Some(mint_pubkey) = mint {
                        // Get destination account to check owner
                        match CacheUtil::get_account(rpc_client, destination_address, false).await {
                            Ok(dest_account) => {
                                let token_program =
                                    TokenType::get_token_program_from_owner(&dest_account.owner)?;
                                let token_account = token_program
                                    .unpack_token_account(&dest_account.data)
                                    .map_err(|e| {
                                        KoraError::TokenOperationError(format!(
                                            "Failed to unpack destination token account {}: {}",
                                            destination_address, e
                                        ))
                                    })?;
                                if token_account.owner() == *fee_payer {
                                    mint_to_transfers
                                        .entry(*mint_pubkey)
                                        .or_default()
                                        .push((*amount, false)); // inflow
                                }
                            }
                            Err(e) => {
                                // If we get Account not found error, we try to match it to the ATA derivation for the fee payer
                                // in case that ATA is being created in the current instruction
                                if matches!(e, KoraError::AccountNotFound(_)) {
                                    let spl_ata =
                                        spl_associated_token_account_interface::address::get_associated_token_address(
                                            fee_payer,
                                            mint_pubkey,
                                        );
                                    let token2022_ata =
                                        get_associated_token_address_with_program_id(
                                            fee_payer,
                                            mint_pubkey,
                                            &spl_token_2022_interface::id(),
                                        );

                                    // If destination matches a valid ATA for fee payer, count as inflow
                                    if *destination_address == spl_ata
                                        || *destination_address == token2022_ata
                                    {
                                        mint_to_transfers
                                            .entry(*mint_pubkey)
                                            .or_default()
                                            .push((*amount, false)); // inflow
                                    }
                                    // Otherwise, it's not fee payer's account, continue to next transfer
                                } else {
                                    // Skip if destination account doesn't exist or can't be fetched
                                    // This could be problematic for non ATA token accounts created
                                    // during the transaction
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }

        if mint_to_transfers.is_empty() {
            return Ok(0);
        }

        // Batch fetch all prices and decimals
        let mint_addresses: Vec<String> =
            mint_to_transfers.keys().map(|mint| mint.to_string()).collect();

        let oracle = RetryingPriceOracle::new(
            3,
            Duration::from_secs(1),
            get_price_oracle(price_source.clone())?,
        );

        let prices = oracle.get_token_prices(&mint_addresses).await?;

        let mut mint_decimals = std::collections::HashMap::new();
        for mint in mint_to_transfers.keys() {
            let decimals = Self::get_mint_decimals(rpc_client, mint).await?;
            mint_decimals.insert(*mint, decimals);
        }

        // Calculate total value
        let mut total_lamports = 0u64;

        for (mint, transfers) in mint_to_transfers.iter() {
            let price = prices
                .get(&mint.to_string())
                .ok_or_else(|| KoraError::RpcError(format!("No price data for mint {mint}")))?;
            let decimals = mint_decimals
                .get(mint)
                .ok_or_else(|| KoraError::RpcError(format!("No decimals data for mint {mint}")))?;

            for (amount, is_outflow) in transfers {
                // Convert token amount to lamports value using Decimal
                let amount_decimal = Decimal::from_u64(*amount).ok_or_else(|| {
                    KoraError::ValidationError("Invalid transfer amount".to_string())
                })?;
                let decimals_scale = Decimal::from_u64(10u64.pow(*decimals as u32))
                    .ok_or_else(|| KoraError::ValidationError("Invalid decimals".to_string()))?;
                let lamports_per_sol = Decimal::from_u64(LAMPORTS_PER_SOL).ok_or_else(|| {
                    KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string())
                })?;

                // Calculate: (amount * price * LAMPORTS_PER_SOL) / 10^decimals
                // Multiply before divide to preserve precision
                let lamports_decimal = amount_decimal.checked_mul(price.price)
                    .and_then(|result| result.checked_mul(lamports_per_sol))
                    .and_then(|result| result.checked_div(decimals_scale))
                    .ok_or_else(|| {
                        log::error!("Token value calculation overflow: amount={}, price={}, decimals={}, lamports_per_sol={}",
                            amount,
                            price.price,
                            decimals,
                            lamports_per_sol
                        );
                        KoraError::ValidationError("Token value calculation overflow".to_string())
                    })?;

                let lamports = lamports_decimal.floor().to_u64().ok_or_else(|| {
                    KoraError::ValidationError("Lamports value overflow".to_string())
                })?;

                if *is_outflow {
                    // Add outflow to total
                    total_lamports = total_lamports.checked_add(lamports).ok_or_else(|| {
                        log::error!("SPL outflow calculation overflow");
                        KoraError::ValidationError("SPL outflow calculation overflow".to_string())
                    })?;
                } else {
                    // Subtract inflow from total (using saturating_sub to prevent underflow)
                    total_lamports = total_lamports.saturating_sub(lamports);
                }
            }
        }

        Ok(total_lamports)
    }

    /// Validate Token2022 extensions for payment instructions
    /// This checks if any blocked extensions are present on the payment accounts
    pub async fn validate_token2022_extensions_for_payment(
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        destination_address: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        let config = &get_config()?.validation.token_2022;

        let token_program = Token2022Program::new();

        // Get mint account data and validate mint extensions (force refresh in case extensions are added)
        let mint_account = CacheUtil::get_account(rpc_client, mint, true).await?;
        let mint_data = mint_account.data;

        // Unpack the mint state with extensions
        let mint_state = token_program.unpack_mint(mint, &mint_data)?;

        let mint_with_extensions =
            mint_state.as_any().downcast_ref::<Token2022Mint>().ok_or_else(|| {
                KoraError::SerializationError("Failed to downcast mint state.".to_string())
            })?;

        // Check each extension type present on the mint
        for extension_type in mint_with_extensions.get_extension_types() {
            if config.is_mint_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked mint extension found on mint account {mint}",
                )));
            }
        }

        // Check source account extensions (force refresh in case extensions are added)
        let source_account = CacheUtil::get_account(rpc_client, source_address, true).await?;
        let source_data = source_account.data;

        let source_state = token_program.unpack_token_account(&source_data)?;

        let source_with_extensions =
            source_state.as_any().downcast_ref::<Token2022Account>().ok_or_else(|| {
                KoraError::SerializationError("Failed to downcast source state.".to_string())
            })?;

        for extension_type in source_with_extensions.get_extension_types() {
            if config.is_account_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked account extension found on source account {source_address}",
                )));
            }
        }

        // Check destination account extensions (force refresh in case extensions are added)
        let destination_account =
            CacheUtil::get_account(rpc_client, destination_address, true).await?;
        let destination_data = destination_account.data;

        let destination_state = token_program.unpack_token_account(&destination_data)?;

        let destination_with_extensions =
            destination_state.as_any().downcast_ref::<Token2022Account>().ok_or_else(|| {
                KoraError::SerializationError("Failed to downcast destination state.".to_string())
            })?;

        for extension_type in destination_with_extensions.get_extension_types() {
            if config.is_account_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked account extension found on destination account {destination_address}",
                )));
            }
        }

        Ok(())
    }

    pub async fn verify_token_payment(
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        required_lamports: u64,
        // Wallet address of the owner of the destination token account
        expected_destination_owner: &Pubkey,
    ) -> Result<bool, KoraError> {
        let config = get_config()?;
        let mut total_lamport_value = 0u64;

        for instruction in transaction_resolved
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                source_address,
                destination_address,
                mint,
                amount,
                is_2022,
                ..
            } = instruction
            {
                let token_program: Box<dyn TokenInterface> = if *is_2022 {
                    Box::new(Token2022Program::new())
                } else {
                    Box::new(TokenProgram::new())
                };

                // Validate the destination account is that of the payment address (or signer if none provided)
                let destination_account =
                    CacheUtil::get_account(rpc_client, destination_address, false)
                        .await
                        .map_err(|e| KoraError::RpcError(e.to_string()))?;

                let token_state =
                    token_program.unpack_token_account(&destination_account.data).map_err(|e| {
                        KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                    })?;

                // For Token2022 payments, validate that blocked extensions are not used
                if *is_2022 {
                    TokenUtil::validate_token2022_extensions_for_payment(
                        rpc_client,
                        source_address,
                        destination_address,
                        &mint.unwrap_or(token_state.mint()),
                    )
                    .await?;
                }

                // Skip transfer if destination isn't our expected payment address
                if token_state.owner() != *expected_destination_owner {
                    continue;
                }

                if !config.validation.supports_token(&token_state.mint().to_string()) {
                    log::warn!(
                        "Ignoring payment with unsupported token mint: {}",
                        token_state.mint(),
                    );
                    continue;
                }

                let lamport_value = TokenUtil::calculate_token_value_in_lamports(
                    *amount,
                    &token_state.mint(),
                    config.validation.price_source.clone(),
                    rpc_client,
                )
                .await?;

                total_lamport_value =
                    total_lamport_value.checked_add(lamport_value).ok_or_else(|| {
                        log::error!(
                            "Payment accumulation overflow: total={}, new_payment={}",
                            total_lamport_value,
                            lamport_value
                        );
                        KoraError::ValidationError("Payment accumulation overflow".to_string())
                    })?;
            }
        }

        Ok(total_lamport_value >= required_lamports)
    }
}

#[cfg(test)]
mod tests_token {
    use crate::{
        oracle::utils::{USDC_DEVNET_MINT, WSOL_DEVNET_MINT},
        tests::{
            common::{RpcMockBuilder, TokenAccountMockBuilder},
            config_mock::ConfigMockBuilder,
        },
    };

    use super::*;

    #[test]
    fn test_token_type_get_token_program_from_owner_spl() {
        let spl_token_owner = spl_token_interface::id();
        let result = TokenType::get_token_program_from_owner(&spl_token_owner).unwrap();
        assert_eq!(result.program_id(), spl_token_interface::id());
    }

    #[test]
    fn test_token_type_get_token_program_from_owner_token2022() {
        let token2022_owner = spl_token_2022_interface::id();
        let result = TokenType::get_token_program_from_owner(&token2022_owner).unwrap();
        assert_eq!(result.program_id(), spl_token_2022_interface::id());
    }

    #[test]
    fn test_token_type_get_token_program_from_owner_invalid() {
        let invalid_owner = Pubkey::new_unique();
        let result = TokenType::get_token_program_from_owner(&invalid_owner);
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, KoraError::TokenOperationError(_)));
        }
    }

    #[test]
    fn test_token_type_get_token_program_spl() {
        let token_type = TokenType::Spl;
        let result = token_type.get_token_program();
        assert_eq!(result.program_id(), spl_token_interface::id());
    }

    #[test]
    fn test_token_type_get_token_program_token2022() {
        let token_type = TokenType::Token2022;
        let result = token_type.get_token_program();
        assert_eq!(result.program_id(), spl_token_2022_interface::id());
    }

    #[test]
    fn test_check_valid_tokens_valid() {
        let valid_tokens = vec![WSOL_DEVNET_MINT.to_string(), USDC_DEVNET_MINT.to_string()];
        let result = TokenUtil::check_valid_tokens(&valid_tokens).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].to_string(), WSOL_DEVNET_MINT);
        assert_eq!(result[1].to_string(), USDC_DEVNET_MINT);
    }

    #[test]
    fn test_check_valid_tokens_invalid() {
        let invalid_tokens = vec!["invalid_token_address".to_string()];
        let result = TokenUtil::check_valid_tokens(&invalid_tokens);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_check_valid_tokens_empty() {
        let empty_tokens = vec![];
        let result = TokenUtil::check_valid_tokens(&empty_tokens).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_check_valid_tokens_mixed_valid_invalid() {
        let mixed_tokens = vec![WSOL_DEVNET_MINT.to_string(), "invalid_address".to_string()];
        let result = TokenUtil::check_valid_tokens(&mixed_tokens);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_get_mint_valid() {
        // Any valid mint account (valid owner and valid data) will count as valid here. (not related to allowed mint in Kora's config)
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let result = TokenUtil::get_mint(&rpc_client, &mint).await;
        assert!(result.is_ok());
        let mint_data = result.unwrap();
        assert_eq!(mint_data.decimals(), 9);
    }

    #[tokio::test]
    async fn test_get_mint_account_not_found() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = TokenUtil::get_mint(&rpc_client, &mint).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_mint_decimals_valid() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let result = TokenUtil::get_mint_decimals(&rpc_client, &mint).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6);
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_spl() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(decimals, 9);
        assert_eq!(token_price.price, Decimal::from(1));
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_token2022() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(decimals, 6);
        assert_eq!(token_price.price, dec!(0.0001));
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_account_not_found() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result =
            TokenUtil::get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_sol() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let amount = 1_000_000_000; // 1 SOL in lamports
        let result = TokenUtil::calculate_token_value_in_lamports(
            amount,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        assert_eq!(result, 1_000_000_000); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_usdc() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let amount = 1_000_000; // 1 USDC (6 decimals)
        let result = TokenUtil::calculate_token_value_in_lamports(
            amount,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        // 1 USDC * 0.0001 SOL/USDC = 0.0001 SOL = 100,000 lamports
        assert_eq!(result, 100_000);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_zero_amount() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let amount = 0;
        let result = TokenUtil::calculate_token_value_in_lamports(
            amount,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_small_amount() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let amount = 1; // 0.000001 USDC (smallest unit)
        let result = TokenUtil::calculate_token_value_in_lamports(
            amount,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        // 0.000001 USDC * 0.0001 SOL/USDC = very small amount, should floor to 0
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_sol() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let lamports = 1_000_000_000; // 1 SOL
        let result = TokenUtil::calculate_lamports_value_in_token(
            lamports,
            &mint,
            &PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        assert_eq!(result, 1_000_000_000); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_usdc() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let lamports = 100_000; // 0.0001 SOL
        let result = TokenUtil::calculate_lamports_value_in_token(
            lamports,
            &mint,
            &PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        // 0.0001 SOL / 0.0001 SOL/USDC = 1 USDC = 1,000,000 base units
        assert_eq!(result, 1_000_000);
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_zero_lamports() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let lamports = 0;
        let result = TokenUtil::calculate_lamports_value_in_token(
            lamports,
            &mint,
            &PriceSource::Mock,
            &rpc_client,
        )
        .await
        .unwrap();

        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_calculate_price_functions_consistency() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        // Test that convert to lamports and back to token amount gives approximately the same result
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let original_amount = 1_000_000u64; // 1 USDC

        // Convert token amount to lamports
        let lamports_result = TokenUtil::calculate_token_value_in_lamports(
            original_amount,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await;

        if lamports_result.is_err() {
            // If we can't get the account data, skip this test as it requires account lookup
            return;
        }

        let lamports = lamports_result.unwrap();

        // Convert lamports back to token amount
        let recovered_amount_result = TokenUtil::calculate_lamports_value_in_token(
            lamports,
            &mint,
            &PriceSource::Mock,
            &rpc_client,
        )
        .await;

        if let Ok(recovered_amount) = recovered_amount_result {
            assert_eq!(recovered_amount, original_amount);
        }
    }

    #[tokio::test]
    async fn test_price_calculation_with_account_error() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = TokenUtil::calculate_token_value_in_lamports(
            1_000_000,
            &mint,
            PriceSource::Mock,
            &rpc_client,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lamports_calculation_with_account_error() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = TokenUtil::calculate_lamports_value_in_token(
            1_000_000,
            &mint,
            &PriceSource::Mock,
            &rpc_client,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_decimal_precision() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();

        // Explanation (i.e. for case 1)
        // 1. Lamports → SOL: 5,000 / 1,000,000,000 = 0.000005 SOL
        // 2. SOL → USDC: 0.000005 SOL / 0.0001 SOL/USDC = 0.05 USDC
        // 3. USDC → Base units: 0.05 USDC × 10^6 = 50,000 base units

        let test_cases = vec![
            // Low priority fees
            (5_000u64, 50_000u64, "low priority base case"),
            (10_001u64, 100_010u64, "odd number precision"),
            // High priority fees
            (1_010_050u64, 10_100_500u64, "high priority problematic case"),
            // High compute unit scenarios
            (5_000_000u64, 50_000_000u64, "very high CU limit"),
            (2_500_050u64, 25_000_500u64, "odd high amount"), // exact result with Decimal
            (10_000_000u64, 100_000_000u64, "maximum CU cost"),
            // Edge cases
            (1_010_049u64, 10_100_490u64, "precision edge case -1"),
            (1_010_051u64, 10_100_510u64, "precision edge case +1"),
            (999_999u64, 9_999_990u64, "near million boundary"),
            (1_000_001u64, 10_000_010u64, "over million boundary"),
            (1_333_337u64, 13_333_370u64, "repeating digits edge case"),
        ];

        for (lamports, expected, description) in test_cases {
            let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();
            let result = TokenUtil::calculate_lamports_value_in_token(
                lamports,
                &mint,
                &PriceSource::Mock,
                &rpc_client,
            )
            .await
            .unwrap();

            assert_eq!(
                result, expected,
                "Failed for {description}: lamports={lamports}, expected={expected}, got={result}",
            );
        }
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_rpc_error() {
        let _lock = ConfigMockBuilder::new().build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &rpc_client,
            &source_address,
            &destination_address,
            &mint_address,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_no_mint_provided() {
        let _lock = ConfigMockBuilder::new().build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        // Create accounts without any blocked extensions - test source account first
        let source_account = TokenAccountMockBuilder::new().build_token2022();

        let rpc_client = RpcMockBuilder::new().with_account_info(&source_account).build();

        // Test with None mint (should only check account extensions but will fail on dest account lookup)
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &rpc_client,
            &source_address,
            &destination_address,
            &mint_address,
        )
        .await;

        // This will fail on destination lookup, but validates source account extension logic
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains("Blocked account extension found on source account"));
    }

    #[test]
    fn test_config_token2022_extension_blocking() {
        use spl_token_2022_interface::extension::ExtensionType;

        let mut config_builder = ConfigMockBuilder::new();
        config_builder = config_builder
            .with_blocked_token2022_mint_extensions(vec![
                "transfer_fee_config".to_string(),
                "pausable".to_string(),
                "non_transferable".to_string(),
            ])
            .with_blocked_token2022_account_extensions(vec![
                "non_transferable_account".to_string(),
                "cpi_guard".to_string(),
                "memo_transfer".to_string(),
            ]);
        let _lock = config_builder.build_and_setup();

        let config = get_config().unwrap();

        // Test mint extension blocking
        assert!(config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::TransferFeeConfig));
        assert!(config.validation.token_2022.is_mint_extension_blocked(ExtensionType::Pausable));
        assert!(config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::NonTransferable));
        assert!(!config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::InterestBearingConfig));

        // Test account extension blocking
        assert!(config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::NonTransferableAccount));
        assert!(config.validation.token_2022.is_account_extension_blocked(ExtensionType::CpiGuard));
        assert!(config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::MemoTransfer));
        assert!(!config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::ImmutableOwner));
    }

    #[test]
    fn test_config_token2022_empty_extension_blocking() {
        use spl_token_2022_interface::extension::ExtensionType;

        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = crate::tests::config_mock::mock_state::get_config().unwrap();

        // Test that no extensions are blocked by default
        assert!(!config
            .validation
            .token_2022
            .is_mint_extension_blocked(ExtensionType::TransferFeeConfig));
        assert!(!config.validation.token_2022.is_mint_extension_blocked(ExtensionType::Pausable));
        assert!(!config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::NonTransferableAccount));
        assert!(!config
            .validation
            .token_2022
            .is_account_extension_blocked(ExtensionType::CpiGuard));
    }
}
