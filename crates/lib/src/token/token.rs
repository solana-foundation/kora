use crate::{
    config::Config,
    error::KoraError,
    token::{
        interface::TokenMint, spl_token::TokenProgram, spl_token_2022::Token2022Program,
        TokenInterface,
    },
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

mod ata;
mod payment;
mod pricing;
mod transfer_hook;

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PaymentLamportTotals {
    pub(crate) inflow: u64,
    pub(crate) outflow: u64,
}

impl PaymentLamportTotals {
    pub(crate) fn net_payment(self) -> u64 {
        if self.outflow > self.inflow {
            log::warn!(
                "Net-negative payment detected: inflow={}, outflow={}",
                self.inflow,
                self.outflow
            );
        }
        self.inflow.saturating_sub(self.outflow)
    }

    pub(crate) fn checked_add_assign(&mut self, other: Self) -> Result<(), KoraError> {
        self.inflow = self.inflow.checked_add(other.inflow).ok_or_else(|| {
            log::error!(
                "Payment inflow accumulation overflow: total={}, new_payment={}",
                self.inflow,
                other.inflow
            );
            KoraError::ValidationError("Payment inflow accumulation overflow".to_string())
        })?;

        self.outflow = self.outflow.checked_add(other.outflow).ok_or_else(|| {
            log::error!(
                "Payment outflow accumulation overflow: total={}, new_payment={}",
                self.outflow,
                other.outflow
            );
            KoraError::ValidationError("Payment outflow accumulation overflow".to_string())
        })?;

        Ok(())
    }
}

pub struct TokenUtil;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferHookValidationFlow {
    DelayedSigning,
    ImmediateSignAndSend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtaCreationInstructionInfo {
    pub payer: Pubkey,
    pub ata_address: Pubkey,
    pub wallet_owner: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub is_idempotent: bool,
}

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
        config: &Config,
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        let mint_account = CacheUtil::get_account(config, rpc_client, mint_pubkey, false).await?;

        let token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;

        token_program
            .unpack_mint(mint_pubkey, &mint_account.data)
            .map_err(|e| KoraError::TokenOperationError(format!("Failed to unpack mint: {e}")))
    }

    pub async fn get_mint_decimals(
        config: &Config,
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<u8, KoraError> {
        let mint = Self::get_mint(config, rpc_client, mint_pubkey).await?;
        Ok(mint.decimals())
    }
}

#[cfg(test)]
mod tests_token {
    use crate::{
        oracle::{
            utils::{USDC_DEVNET_MINT, WSOL_DEVNET_MINT},
            PriceSource, TokenPrice,
        },
        tests::{
            account_mock::create_transfer_fee_config,
            common::{MintAccountMockBuilder, RpcMockBuilder, TokenAccountMockBuilder},
            config_mock::ConfigMockBuilder,
            oracle_mock::TEST_ORACLE,
        },
        transaction::{ParsedSPLInstructionData, TransactionUtil},
    };
    use rust_decimal::Decimal;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::instruction::Instruction;
    use spl_associated_token_account_interface::program::id as ata_program_id;
    use spl_token_2022_interface::{
        extension::{
            transfer_fee::TransferFeeConfig, BaseStateWithExtensionsMut, ExtensionType,
            PodStateWithExtensionsMut,
        },
        pod::PodMint,
    };
    use std::{collections::HashMap, sync::Arc};

    use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;

    use super::*;

    fn create_token2022_mint_account_with_transfer_fee(
        decimals: u8,
        basis_points: u16,
        max_fee: u64,
    ) -> solana_sdk::account::Account {
        let mut mint_account = MintAccountMockBuilder::new()
            .with_decimals(decimals)
            .with_extension(ExtensionType::TransferFeeConfig)
            .build_token2022();

        let mut mint_state = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_account.data)
            .expect("Failed to unpack Token2022 mint state");
        let transfer_fee_config = mint_state
            .get_extension_mut::<TransferFeeConfig>()
            .expect("Failed to get mutable TransferFeeConfig extension");
        *transfer_fee_config = create_transfer_fee_config(basis_points, max_fee);

        mint_account
    }

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

        let config = get_config().unwrap();
        let result = TokenUtil::get_mint(&config, &rpc_client, &mint).await;
        assert!(result.is_ok());
        let mint_data = result.unwrap();
        assert_eq!(mint_data.decimals(), 9);
    }

    #[tokio::test]
    async fn test_get_mint_account_not_found() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let config = get_config().unwrap();
        let result = TokenUtil::get_mint(&config, &rpc_client, &mint).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_mint_decimals_valid() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let config = get_config().unwrap();
        let result = TokenUtil::get_mint_decimals(&config, &rpc_client, &mint).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6);
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_spl() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, &rpc_client, &config).await.unwrap();

        assert_eq!(decimals, 9);
        assert_eq!(token_price.price, Decimal::from(1));
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_token2022() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let (token_price, decimals) =
            TokenUtil::get_token_price_and_decimals(&mint, &rpc_client, &config).await.unwrap();

        assert_eq!(decimals, 6);
        assert_eq!(token_price.price, dec!(0.0075));
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_account_not_found() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = TokenUtil::get_token_price_and_decimals(&mint, &rpc_client, &config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_payment_lamport_totals_net_payment() {
        assert_eq!(PaymentLamportTotals { inflow: 10, outflow: 4 }.net_payment(), 6);
        assert_eq!(PaymentLamportTotals { inflow: 4, outflow: 4 }.net_payment(), 0);
        assert_eq!(PaymentLamportTotals { inflow: 4, outflow: 10 }.net_payment(), 0);
    }

    #[test]
    fn test_payment_lamport_totals_checked_add_assign() {
        let mut totals = PaymentLamportTotals { inflow: 3, outflow: 2 };
        totals.checked_add_assign(PaymentLamportTotals { inflow: 5, outflow: 7 }).unwrap();

        assert_eq!(totals, PaymentLamportTotals { inflow: 8, outflow: 9 });
    }

    #[test]
    fn test_payment_lamport_totals_checked_add_assign_overflow() {
        let mut totals = PaymentLamportTotals { inflow: u64::MAX, outflow: 0 };
        let err = totals.checked_add_assign(PaymentLamportTotals { inflow: 1, outflow: 0 });
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_sol() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let amount = 1_000_000_000; // 1 SOL in lamports
        let result =
            TokenUtil::calculate_token_value_in_lamports(amount, &mint, &rpc_client, &config)
                .await
                .unwrap();

        assert_eq!(result, 1_000_000_000); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_usdc() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let amount = 1_000_000; // 1 USDC (6 decimals)
        let result =
            TokenUtil::calculate_token_value_in_lamports(amount, &mint, &rpc_client, &config)
                .await
                .unwrap();

        // 1 USDC * 0.0075 SOL/USDC = 0.0075 SOL = 7,500,000 lamports
        assert_eq!(result, 7_500_000);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_zero_amount() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let amount = 0;
        let result =
            TokenUtil::calculate_token_value_in_lamports(amount, &mint, &rpc_client, &config)
                .await
                .unwrap();

        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_small_amount() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let amount = 1; // 0.000001 USDC (smallest unit)
        let result =
            TokenUtil::calculate_token_value_in_lamports(amount, &mint, &rpc_client, &config)
                .await
                .unwrap();

        // 0.000001 USDC * 0.0075 SOL/USDC = 0.0000000075 SOL = 7.5 lamports, floors to 7
        assert_eq!(result, 7);
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_sol() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let lamports = 1_000_000_000; // 1 SOL
        let result =
            TokenUtil::calculate_lamports_value_in_token(lamports, &mint, &rpc_client, &config)
                .await
                .unwrap();

        assert_eq!(result, 1_000_000_000); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_usdc() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let lamports = 7_500_000; // 0.0075 SOL
        let result =
            TokenUtil::calculate_lamports_value_in_token(lamports, &mint, &rpc_client, &config)
                .await
                .unwrap();

        // 0.0075 SOL / 0.0075 SOL/USDC = 1 USDC = 1,000,000 base units
        assert_eq!(result, 1_000_000);
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_zero_lamports() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(9).build();

        let lamports = 0;
        let result =
            TokenUtil::calculate_lamports_value_in_token(lamports, &mint, &rpc_client, &config)
                .await
                .unwrap();

        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_calculate_price_functions_consistency() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        // Test that convert to lamports and back to token amount gives approximately the same result
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let original_amount = 1_000_000u64; // 1 USDC

        // Convert token amount to lamports
        let lamports_result = TokenUtil::calculate_token_value_in_lamports(
            original_amount,
            &mint,
            &rpc_client,
            &config,
        )
        .await;

        if lamports_result.is_err() {
            // If we can't get the account data, skip this test as it requires account lookup
            return;
        }

        let lamports = lamports_result.unwrap();

        // Convert lamports back to token amount
        let recovered_amount_result =
            TokenUtil::calculate_lamports_value_in_token(lamports, &mint, &rpc_client, &config)
                .await;

        if let Ok(recovered_amount) = recovered_amount_result {
            assert_eq!(recovered_amount, original_amount);
        }
    }

    #[tokio::test]
    async fn test_calculate_spl_transfers_value_in_lamports_ignores_inflows() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let config = get_config().unwrap();

        let fee_payer = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let amount = 100_000_000u64; // 100 USDC

        let fee_payer_token2022_ata = get_associated_token_address_with_program_id(
            &fee_payer,
            &mint,
            &spl_token_2022_interface::id(),
        );

        // One outflow from fee payer, one inflow to fee payer — only outflow should be counted
        let spl_transfers = vec![
            ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner: fee_payer,
                multisig_signers: vec![],
                mint: Some(mint),
                source_address: Pubkey::new_unique(),
                destination_address: other,
                is_2022: true,
            },
            ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner: other,
                multisig_signers: vec![],
                mint: Some(mint),
                source_address: Pubkey::new_unique(),
                destination_address: fee_payer_token2022_ata,
                is_2022: true,
            },
        ];

        let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();

        let spl_outflow_value = TokenUtil::calculate_spl_transfers_value_in_lamports(
            &spl_transfers,
            &fee_payer,
            &rpc_client,
            &config,
        )
        .await
        .unwrap();

        // Only fee payer's outflow of 100 USDC is counted; the inflow is ignored.
        // 100 USDC at mock price = 750_000_000 lamports.
        assert_eq!(spl_outflow_value, 750_000_000);
    }

    #[tokio::test]
    async fn test_calculate_spl_transfers_value_in_lamports_token2022_outflow_remains_gross() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let config = get_config().unwrap();

        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let amount = 100_000_000u64; // 100 USDC

        let spl_transfers = vec![ParsedSPLInstructionData::SplTokenTransfer {
            amount,
            owner: fee_payer,
            multisig_signers: vec![],
            mint: Some(mint),
            source_address: Pubkey::new_unique(),
            destination_address: Pubkey::new_unique(),
            is_2022: true,
        }];

        let mint_account = create_token2022_mint_account_with_transfer_fee(6, 100, 1_000_000);
        let rpc_client = RpcMockBuilder::new().build_with_sequential_accounts(vec![&mint_account]);

        let spl_outflow_value = TokenUtil::calculate_spl_transfers_value_in_lamports(
            &spl_transfers,
            &fee_payer,
            &rpc_client,
            &config,
        )
        .await
        .unwrap();

        // Outflow should still be charged at transfer gross amount (100 USDC).
        assert_eq!(spl_outflow_value, 750_000_000);
    }

    #[tokio::test]
    async fn test_price_calculation_with_account_error() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result =
            TokenUtil::calculate_token_value_in_lamports(1_000_000, &mint, &rpc_client, &config)
                .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lamports_calculation_with_account_error() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result =
            TokenUtil::calculate_lamports_value_in_token(1_000_000, &mint, &rpc_client, &config)
                .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_decimal_precision() {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let config = get_config().unwrap();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();

        // Formula: ceil((lamports / LAMPORTS_PER_SOL) / price * 10^decimals)
        let test_cases = vec![
            // Low priority fees
            (5_000u64, 667u64, "low priority base case"),
            (10_001u64, 1_334u64, "odd number precision"),
            // High priority fees
            (1_010_050u64, 134_674u64, "high priority problematic case"),
            // High compute unit scenarios
            (5_000_000u64, 666_667u64, "very high CU limit"),
            (2_500_050u64, 333_340u64, "odd high amount"),
            (10_000_000u64, 1_333_334u64, "maximum CU cost"),
            // Edge cases
            (1_010_049u64, 134_674u64, "precision edge case -1"),
            (1_010_051u64, 134_674u64, "precision edge case +1"),
            (999_999u64, 133_334u64, "near million boundary"),
            (1_000_001u64, 133_334u64, "over million boundary"),
            (1_333_337u64, 177_779u64, "repeating digits edge case"),
        ];

        for (lamports, expected, description) in test_cases {
            let rpc_client = RpcMockBuilder::new().with_mint_account(6).build();
            let result =
                TokenUtil::calculate_lamports_value_in_token(lamports, &mint, &rpc_client, &config)
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

        let config = get_config().unwrap();
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &config,
            &rpc_client,
            &source_address,
            &destination_address,
            &mint_address,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_allows_mutable_transfer_hook_authority()
    {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(6)
            .with_extension(spl_token_2022_interface::extension::ExtensionType::TransferHook)
            .with_transfer_hook_authority(Some(Pubkey::new_unique()))
            .with_transfer_hook_program_id(Some(Pubkey::new_unique()))
            .build_token2022();
        let source_account =
            TokenAccountMockBuilder::new().with_mint(&mint_address).build_token2022();
        let destination_account =
            TokenAccountMockBuilder::new().with_mint(&mint_address).build_token2022();

        let rpc_client = RpcMockBuilder::new().build_with_sequential_accounts(vec![
            &mint_account,
            &source_account,
            &destination_account,
        ]);

        let config = get_config().unwrap();
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &config,
            &rpc_client,
            &source_address,
            &destination_address,
            &mint_address,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_allows_immutable_transfer_hook_authority(
    ) {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(6)
            .with_extension(spl_token_2022_interface::extension::ExtensionType::TransferHook)
            .with_transfer_hook_authority(None)
            .with_transfer_hook_program_id(Some(Pubkey::new_unique()))
            .build_token2022();
        let source_account =
            TokenAccountMockBuilder::new().with_mint(&mint_address).build_token2022();
        let destination_account =
            TokenAccountMockBuilder::new().with_mint(&mint_address).build_token2022();

        let rpc_client = RpcMockBuilder::new().build_with_sequential_accounts(vec![
            &mint_account,
            &source_account,
            &destination_account,
        ]);

        let config = get_config().unwrap();
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &config,
            &rpc_client,
            &source_address,
            &destination_address,
            &mint_address,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_token2022_partial_for_ata_creation_allows_mutable_transfer_hook_authority(
    ) {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let source_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(6)
            .with_extension(spl_token_2022_interface::extension::ExtensionType::TransferHook)
            .with_transfer_hook_authority(Some(Pubkey::new_unique()))
            .with_transfer_hook_program_id(Some(Pubkey::new_unique()))
            .build_token2022();
        let source_account =
            TokenAccountMockBuilder::new().with_mint(&mint_address).build_token2022();

        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&mint_account, &source_account]);

        let config = get_config().unwrap();
        let result = TokenUtil::validate_token2022_partial_for_ata_creation(
            &config,
            &rpc_client,
            &source_address,
            &mint_address,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_token2022_extensions_for_payment_no_mint_provided() {
        let _lock = ConfigMockBuilder::new().build_and_setup();

        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint_address = Pubkey::new_unique();

        let source_account = TokenAccountMockBuilder::new().build_token2022();

        let rpc_client = RpcMockBuilder::new().with_account_info(&source_account).build();

        // Test with None mint (should only check account extensions but will fail on dest account lookup)
        let config = get_config().unwrap();
        let result = TokenUtil::validate_token2022_extensions_for_payment(
            &config,
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

    #[tokio::test]
    async fn test_calculate_payment_lamport_totals_validates_token2022_when_both_sides_match_expected_owner(
    ) {
        let _lock = ConfigMockBuilder::new().build_and_setup();
        let mut config = get_config().unwrap();
        config.kora.cache.enabled = false;
        config.validation.token_2022.blocked_mint_extensions =
            vec!["transfer_fee_config".to_string()];
        let _ = config.validation.token_2022.initialize();

        let expected_destination_owner = Pubkey::new_unique();
        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();

        let source_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&expected_destination_owner)
            .build_token2022();
        let destination_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&expected_destination_owner)
            .build_token2022();
        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(6)
            .with_extension(ExtensionType::TransferFeeConfig)
            .build_token2022();

        let instruction = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &source_address,
            &mint,
            &destination_address,
            &expected_destination_owner,
            &[],
            1_000_000,
            6,
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(
            &[instruction],
            Some(&expected_destination_owner),
        ));
        let mut transaction_resolved =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let rpc_client = RpcMockBuilder::new().build_with_sequential_accounts(vec![
            &source_account,
            &destination_account,
            &mint_account,
            &source_account,
        ]);

        let result = TokenUtil::calculate_payment_lamport_totals(
            &config,
            &mut transaction_resolved,
            &rpc_client,
            &expected_destination_owner,
            None,
        )
        .await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Blocked mint extension found on mint account"));
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

    #[test]
    fn test_find_ata_creation_for_destination_found() {
        use solana_sdk::instruction::AccountMeta;

        let funding_account = Pubkey::new_unique();
        let wallet_owner = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        // Derive the ATA address
        let ata_address =
            spl_associated_token_account_interface::address::get_associated_token_address(
                &wallet_owner,
                &mint,
            );

        // Create a mock ATA creation instruction
        let ata_instruction = Instruction {
            program_id: ata_program_id,
            accounts: vec![
                AccountMeta::new(funding_account, true), // 0: funding account
                AccountMeta::new(ata_address, false),    // 1: ATA to be created
                AccountMeta::new_readonly(wallet_owner, false), // 2: wallet owner
                AccountMeta::new_readonly(mint, false),  // 3: mint
                AccountMeta::new_readonly(solana_system_interface::program::ID, false), // 4: system program
                AccountMeta::new_readonly(spl_token_interface::id(), false), // 5: token program
            ],
            data: vec![0], // CreateAssociatedTokenAccount instruction discriminator
        };

        let instructions = vec![ata_instruction];

        // Should find the ATA creation instruction
        let result = TokenUtil::find_ata_creation_for_destination(&instructions, &ata_address);
        assert!(result.is_some());
        let (found_wallet, found_mint) = result.unwrap();
        assert_eq!(found_wallet, wallet_owner);
        assert_eq!(found_mint, mint);
    }

    #[test]
    fn test_find_ata_creation_for_destination_not_found() {
        use solana_sdk::instruction::AccountMeta;

        let funding_account = Pubkey::new_unique();
        let wallet_owner = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        // Derive the ATA address
        let ata_address =
            spl_associated_token_account_interface::address::get_associated_token_address(
                &wallet_owner,
                &mint,
            );

        let different_ata = Pubkey::new_unique();
        let ata_instruction = Instruction {
            program_id: ata_program_id,
            accounts: vec![
                AccountMeta::new(funding_account, true),
                AccountMeta::new(different_ata, false),
                AccountMeta::new_readonly(wallet_owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
                AccountMeta::new_readonly(spl_token_interface::id(), false),
            ],
            data: vec![0],
        };

        let instructions = vec![ata_instruction];

        // Should NOT find an ATA creation for our target address
        let result = TokenUtil::find_ata_creation_for_destination(&instructions, &ata_address);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_ata_creation_for_destination_empty_instructions() {
        let target_address = Pubkey::new_unique();
        let instructions: Vec<Instruction> = vec![];

        let result = TokenUtil::find_ata_creation_for_destination(&instructions, &target_address);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_ata_creation_for_destination_wrong_program() {
        use solana_sdk::instruction::AccountMeta;

        let target_address = Pubkey::new_unique();
        let wallet_owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let wrong_program_instruction = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), true),
                AccountMeta::new(target_address, false),
                AccountMeta::new_readonly(wallet_owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
                AccountMeta::new_readonly(spl_token_interface::id(), false),
            ],
            data: vec![0],
        };

        let instructions = vec![wrong_program_instruction];

        let result = TokenUtil::find_ata_creation_for_destination(&instructions, &target_address);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ata_creation_instruction_idempotent() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );

        let parsed = TokenUtil::parse_ata_creation_instruction(&ix).expect("ATA should parse");
        assert_eq!(parsed.payer, payer);
        assert_eq!(parsed.wallet_owner, owner);
        assert_eq!(parsed.mint, mint);
        assert_eq!(parsed.token_program, spl_token_interface::id());
        assert!(parsed.is_idempotent);
    }

    #[test]
    fn test_parse_ata_creation_instruction_non_idempotent() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );

        let parsed = TokenUtil::parse_ata_creation_instruction(&ix).expect("ATA should parse");
        assert_eq!(parsed.payer, payer);
        assert_eq!(parsed.wallet_owner, owner);
        assert_eq!(parsed.mint, mint);
        assert_eq!(parsed.token_program, spl_token_interface::id());
        assert!(!parsed.is_idempotent);
    }

    #[test]
    fn test_parse_ata_creation_instruction_empty_data_is_legacy_create() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_address =
            get_associated_token_address_with_program_id(&owner, &mint, &spl_token_interface::id());

        let ix = Instruction {
            program_id: ata_program_id(),
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer, true),
                solana_sdk::instruction::AccountMeta::new(ata_address, false),
                solana_sdk::instruction::AccountMeta::new_readonly(owner, false),
                solana_sdk::instruction::AccountMeta::new_readonly(mint, false),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    solana_system_interface::program::ID,
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    spl_token_interface::id(),
                    false,
                ),
            ],
            data: vec![],
        };

        let parsed = TokenUtil::parse_ata_creation_instruction(&ix).expect("ATA should parse");
        assert_eq!(parsed.payer, payer);
        assert_eq!(parsed.ata_address, ata_address);
        assert_eq!(parsed.wallet_owner, owner);
        assert_eq!(parsed.mint, mint);
        assert_eq!(parsed.token_program, spl_token_interface::id());
        assert!(!parsed.is_idempotent);
    }

    #[test]
    fn test_parse_ata_creation_instruction_rejects_unknown_discriminator() {
        use solana_sdk::instruction::AccountMeta;

        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_address =
            spl_associated_token_account_interface::address::get_associated_token_address(
                &owner, &mint,
            );

        let ix = Instruction {
            program_id: spl_associated_token_account_interface::program::id(),
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(ata_address, false),
                AccountMeta::new_readonly(owner, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
                AccountMeta::new_readonly(spl_token_interface::id(), false),
            ],
            data: vec![2], // Unsupported ATA instruction variant
        };

        assert!(TokenUtil::parse_ata_creation_instruction(&ix).is_none());
    }

    #[test]
    fn test_find_fee_payer_ata_creations_filters_by_payer() {
        let fee_payer = Pubkey::new_unique();
        let other_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();

        let by_fee_payer =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &fee_payer,
                &owner,
                &mint_a,
                &spl_token_interface::id(),
            );
        let by_other_payer =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &other_payer,
                &owner,
                &mint_b,
                &spl_token_interface::id(),
            );

        let parsed =
            TokenUtil::find_fee_payer_ata_creations(&[by_fee_payer, by_other_payer], &fee_payer);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payer, fee_payer);
        assert_eq!(parsed[0].mint, mint_a);
    }

    #[test]
    fn test_decimal_scale_valid_values() {
        use super::pricing::decimal_scale;
        assert_eq!(decimal_scale(0).unwrap(), dec!(1));
        assert_eq!(decimal_scale(6).unwrap(), dec!(1_000_000));
        assert_eq!(decimal_scale(9).unwrap(), dec!(1_000_000_000));
        assert_eq!(decimal_scale(18).unwrap(), dec!(1_000_000_000_000_000_000));
        assert!(decimal_scale(19).is_ok());
    }

    #[test]
    fn test_decimal_scale_rejects_overflow() {
        use super::pricing::decimal_scale;
        let err = decimal_scale(20).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum supported value"));

        let err = decimal_scale(255).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum supported value"));
    }

    #[tokio::test]
    async fn test_calculate_payment_lamport_totals_batches_duplicate_mints() {
        let _lock = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let expected_destination_owner = Pubkey::new_unique();
        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let mint = Pubkey::from_str(USDC_DEVNET_MINT).unwrap();

        let source_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&Pubkey::new_unique())
            .build();
        let destination_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&expected_destination_owner)
            .build();
        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build();

        let instruction1 = spl_token_interface::instruction::transfer_checked(
            &spl_token_interface::id(),
            &source_address,
            &mint,
            &destination_address,
            &expected_destination_owner,
            &[],
            1_000_000,
            6,
        )
        .unwrap();

        let instruction2 = spl_token_interface::instruction::transfer_checked(
            &spl_token_interface::id(),
            &source_address,
            &mint,
            &destination_address,
            &expected_destination_owner,
            &[],
            2_000_000,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[instruction1, instruction2],
            Some(&expected_destination_owner),
        ));
        let mut transaction_resolved =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let rpc_client = RpcMockBuilder::new().build_with_sequential_accounts(vec![
            &source_account,
            &destination_account,
            &source_account,
            &destination_account,
            &mint_account,
        ]);

        let mut mock_oracle = crate::oracle::MockPriceOracle::new();
        mock_oracle.expect_get_prices().times(1).returning(|_, mint_addresses| {
            let mut result = HashMap::new();
            for mint in mint_addresses {
                result.insert(
                    mint.clone(),
                    TokenPrice {
                        price: Decimal::from(1),
                        confidence: 1.0,
                        source: PriceSource::Mock,
                        block_id: None,
                    },
                );
            }
            Ok(result)
        });

        let shared_mock_oracle = Arc::new(mock_oracle);
        TEST_ORACLE.with(|o| {
            *o.borrow_mut() = Some(shared_mock_oracle.clone());
        });

        struct OracleGuard;
        impl Drop for OracleGuard {
            fn drop(&mut self) {
                TEST_ORACLE.with(|o| {
                    *o.borrow_mut() = None;
                });
            }
        }
        let _guard = OracleGuard;

        let mut config = get_config().unwrap();
        config.kora.cache.enabled = false;
        let result = TokenUtil::calculate_payment_lamport_totals(
            &config,
            &mut transaction_resolved,
            &rpc_client,
            &expected_destination_owner,
            None,
        )
        .await;

        assert!(result.is_ok());
        let totals = result.unwrap();
        assert_eq!(totals.inflow, 3_000_000_000);
        assert_eq!(totals.outflow, 0);
    }
}
