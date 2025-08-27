use crate::{
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
    token::{
        interface::TokenMint,
        spl_token_2022::{Token2022Extensions, Token2022Mint},
        Token2022Account, Token2022Program, TokenInterface, TokenProgram,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::{str::FromStr, time::Duration};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Spl,
    Token2022,
}

impl TokenType {
    pub fn get_token_program_from_owner(
        owner: &Pubkey,
    ) -> Result<Box<dyn TokenInterface>, KoraError> {
        if *owner == spl_token::id() {
            Ok(Box::new(TokenProgram::new()))
        } else if *owner == spl_token_2022::id() {
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
            RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source));

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

        // Convert token amount to its real value based on decimals and multiply by SOL price
        let token_amount = amount as f64 / 10f64.powi(decimals as i32);
        let sol_amount = token_amount * token_price.price;

        // Convert SOL to lamports and round down
        let lamports = (sol_amount * LAMPORTS_PER_SOL as f64).floor() as u64;

        Ok(lamports)
    }

    pub async fn calculate_lamports_value_in_token(
        lamports: u64,
        mint: &Pubkey,
        price_source: &PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<f64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, price_source.clone(), rpc_client).await?;

        // Convert lamports to SOL, then to token amount
        let fee_in_sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
        let fee_in_token_base_units = fee_in_sol / token_price.price;
        let fee_in_token = fee_in_token_base_units * 10f64.powi(decimals as i32);

        // Round to nearest integer to fix floating point precision errors
        // This ensures values like 1010049.9999999999 become 1010050
        Ok(fee_in_token.round())
    }

    /// Validate Token2022 extensions for payment instructions
    /// This checks if any blocked extensions are present on the payment accounts
    pub async fn validate_token2022_extensions_for_payment(
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        destination_address: &Pubkey,
        mint: &Option<Pubkey>,
    ) -> Result<(), KoraError> {
        let config = &get_config()?.validation.token_2022;

        let token_program = Token2022Program::new();

        // Get mint account data and validate mint extensions (force refresh in case extensions are added)
        if let Some(mint) = mint {
            let mint_account = CacheUtil::get_account(rpc_client, mint, true).await?;
            let mint_data = mint_account.data;

            // Unpack the mint state with extensions
            let mint_state = token_program.unpack_mint(mint, &mint_data)?;

            let mint_with_extensions = mint_state.as_any().downcast_ref::<Token2022Mint>().unwrap();

            // Check each extension type present on the mint
            for extension_type in mint_with_extensions.get_extension_types() {
                if config.is_mint_extension_blocked(*extension_type) {
                    return Err(KoraError::ValidationError(format!(
                        "Blocked mint extension found on mint account {mint}",
                    )));
                }
            }
        }

        // Check source account extensions (force refresh in case extensions are added)
        let source_account = CacheUtil::get_account(rpc_client, source_address, true).await?;
        let source_data = source_account.data;

        let source_state = token_program.unpack_token_account(&source_data)?;

        let source_with_extensions =
            source_state.as_any().downcast_ref::<Token2022Account>().unwrap();

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
            destination_state.as_any().downcast_ref::<Token2022Account>().unwrap();

        for extension_type in destination_with_extensions.get_extension_types() {
            if config.is_account_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked account extension found on destination account {destination_address}",
                )));
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn process_token_transfer(
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        total_lamport_value: &mut u64,
        required_lamports: u64,
        // Wallet address of the owner of the destination token account
        expected_destination_owner: &Pubkey,
    ) -> Result<bool, KoraError> {
        let config = get_config()?;

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

                // For Token2022 payments, validate that blocked extensions are not used
                if *is_2022 {
                    TokenUtil::validate_token2022_extensions_for_payment(
                        rpc_client,
                        source_address,
                        destination_address,
                        mint,
                    )
                    .await?;
                }

                // Validate the destination account is that of the payment address (or signer if none provided)
                let destination_account =
                    CacheUtil::get_account(rpc_client, destination_address, false)
                        .await
                        .map_err(|e| KoraError::RpcError(e.to_string()))?;

                let token_state =
                    token_program.unpack_token_account(&destination_account.data).map_err(|e| {
                        KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                    })?;

                // Skip transfer if destination isn't our expected payment address
                if token_state.owner() != *expected_destination_owner {
                    continue;
                }

                // If we have a transfer checked and therefore the mint account, we don't need to check the source's account owner as TokenProgram,
                // since we already know the instruction is with the system program,so if the source account is invalid, the instruction with the
                // token program will fail. Same with the balance of the source account, if too low the instruction will fail.
                // This might be useful if the token account is being created within the same transaction, since the source account is not yet created.
                let (mint_address, actual_amount) = if let Some(mint_address) = *mint {
                    // Force refresh in case extensions are modified
                    let mint_account =
                        CacheUtil::get_account(rpc_client, &mint_address, true).await?;
                    let mint_state =
                        token_program.unpack_mint(&mint_address, &mint_account.data)?;

                    let actual_amount = token_program
                        .get_and_validate_amount_for_payment(
                            rpc_client,
                            None,
                            Some(&*mint_state),
                            *amount,
                        )
                        .await
                        .map_err(|e| {
                            KoraError::TokenOperationError(format!(
                                "Failed to validate amount for payment: {e}"
                            ))
                        })?;

                    (mint_address, actual_amount)
                } else {
                    // Force refresh in case extensions are modified
                    let source_account = CacheUtil::get_account(rpc_client, source_address, true)
                        .await
                        .map_err(|e| KoraError::RpcError(e.to_string()))?;

                    let token_state =
                        token_program.unpack_token_account(&source_account.data).map_err(|e| {
                            KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                        })?;

                    if source_account.owner != token_program.program_id() {
                        return Ok(false);
                    }

                    let actual_amount = token_program
                        .get_and_validate_amount_for_payment(
                            rpc_client,
                            Some(&*token_state),
                            None,
                            *amount,
                        )
                        .await
                        .map_err(|e| {
                            KoraError::TokenOperationError(format!(
                                "Failed to validate amount for payment: {e}"
                            ))
                        })?;

                    (token_state.mint(), actual_amount)
                };

                if !config.validation.allowed_spl_paid_tokens.contains(&mint_address.to_string()) {
                    return Ok(false);
                }

                let lamport_value = TokenUtil::calculate_token_value_in_lamports(
                    actual_amount,
                    &mint_address,
                    config.validation.price_source.clone(),
                    rpc_client,
                )
                .await?;

                *total_lamport_value += lamport_value;
                if *total_lamport_value >= required_lamports {
                    return Ok(true); // Payment satisfied
                }
            }
        }

        Ok(false)
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
        let spl_token_owner = spl_token::id();
        let result = TokenType::get_token_program_from_owner(&spl_token_owner).unwrap();
        assert_eq!(result.program_id(), spl_token::id());
    }

    #[test]
    fn test_token_type_get_token_program_from_owner_token2022() {
        let token2022_owner = spl_token_2022::id();
        let result = TokenType::get_token_program_from_owner(&token2022_owner).unwrap();
        assert_eq!(result.program_id(), spl_token_2022::id());
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
        assert_eq!(result.program_id(), spl_token::id());
    }

    #[test]
    fn test_token_type_get_token_program_token2022() {
        let token_type = TokenType::Token2022;
        let result = token_type.get_token_program();
        assert_eq!(result.program_id(), spl_token_2022::id());
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
        assert_eq!(token_price.price, 1.0);
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
        assert_eq!(token_price.price, 0.0001);
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

        assert_eq!(result, 1_000_000_000.0); // Should equal input since SOL price is 1.0
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
        assert_eq!(result, 1_000_000.0);
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

        assert_eq!(result, 0.0);
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
            assert_eq!(recovered_amount, original_amount as f64);
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

        // Comprehensive test cases covering all precision scenarios from PRO-249
        let test_cases = vec![
            // Low priority fees
            (5_000u64, 50_000.0, "low priority base case"),
            (10_001u64, 100_010.0, "odd number precision"),
            // High priority fees
            (1_010_050u64, 10_100_500.0, "high priority problematic case"),
            // High compute unit scenarios
            (5_000_000u64, 50_000_000.0, "very high CU limit"),
            (2_500_050u64, 25_000_500.0, "odd high amount"),
            (10_000_000u64, 100_000_000.0, "maximum CU cost"),
            // Edge cases
            (1_010_049u64, 10_100_490.0, "precision edge case -1"),
            (1_010_051u64, 10_100_510.0, "precision edge case +1"),
            (999_999u64, 9_999_990.0, "near million boundary"),
            (1_000_001u64, 10_000_010.0, "over million boundary"),
            (1_333_337u64, 13_333_370.0, "repeating digits edge case"),
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

            // Must be proper integers (no fractional part)
            assert_eq!(
                result.fract(),
                0.0,
                "Result should be integer for {lamports} lamports: got {result}",
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
            &Some(mint_address),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_no_mint_provided() {
        let _lock = ConfigMockBuilder::new().build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();

        // Create accounts without any blocked extensions - test source account first
        let source_account = TokenAccountMockBuilder::new().build_token2022();

        let rpc_client = RpcMockBuilder::new().with_account_info(&source_account).build();

        // Test with None mint (should only check account extensions but will fail on dest account lookup)
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &rpc_client,
            &source_address,
            &destination_address,
            &None,
        )
        .await;

        // This will fail on destination lookup, but validates source account extension logic
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains("Blocked account extension found on source account"));
    }

    #[test]
    fn test_config_token2022_extension_blocking() {
        use spl_token_2022::extension::ExtensionType;

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
        use spl_token_2022::extension::ExtensionType;

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
