use crate::{
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
    state::get_config,
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
        let mint_account =
            CacheUtil::get_account_from_cache(rpc_client, mint_pubkey, false).await?;

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

        Ok(fee_in_token)
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
            let mint_account = CacheUtil::get_account_from_cache(rpc_client, mint, true).await?;
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
        let source_account =
            CacheUtil::get_account_from_cache(rpc_client, source_address, true).await?;
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
            CacheUtil::get_account_from_cache(rpc_client, destination_address, true).await?;
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
                    CacheUtil::get_account_from_cache(rpc_client, destination_address, false)
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
                        CacheUtil::get_account_from_cache(rpc_client, &mint_address, true).await?;
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
                    let source_account =
                        CacheUtil::get_account_from_cache(rpc_client, source_address, true)
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
    use crate::tests::common::{
        create_mock_spl_mint_account, create_mock_token2022_mint_account, get_mock_rpc_client,
    };

    use super::*;

    #[test]
    fn test_token_program_id() {
        let token_program_id = spl_token::id();
        let token_type = match token_program_id {
            id if id == spl_token::id() => TokenType::Spl,
            id if id == spl_token_2022::id() => TokenType::Token2022,
            _ => panic!("Unknown token program ID"),
        };
        assert_eq!(token_type, TokenType::Spl);
    }

    #[test]
    fn test_token2022_program_id() {
        let token_program_id = spl_token_2022::id();
        let token_22_type = match token_program_id {
            id if id == spl_token::id() => TokenType::Spl,
            id if id == spl_token_2022::id() => TokenType::Token2022,
            _ => panic!("Unknown token program ID"),
        };
        assert_eq!(token_22_type, TokenType::Token2022);
    }

    #[tokio::test]
    async fn test_check_valid_tokens() {
        let valid_tokens = vec![
            "So11111111111111111111111111111111111111112".to_string(),
            "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
        ];
        let result = TokenUtil::check_valid_tokens(&valid_tokens).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].to_string(), "So11111111111111111111111111111111111111112");
        assert_eq!(result[1].to_string(), "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
    }

    #[tokio::test]
    async fn test_check_valid_tokens_invalid() {
        let invalid_tokens = vec!["invalid_token_address".to_string()];
        let result = TokenUtil::check_valid_tokens(&invalid_tokens);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_spl() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(decimals, 9);
        assert_eq!(token_price.price, 1.0);
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_token2022() {
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();

        let account = create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(decimals, 6);
        assert_eq!(token_price.price, 0.0001);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

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
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let account = create_mock_spl_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

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
    async fn test_calculate_lamports_value_in_token() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

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
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let account = create_mock_spl_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

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
}
