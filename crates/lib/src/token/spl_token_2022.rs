use crate::token::{
    interface::TokenMint,
    spl_token_2022_util::{
        try_parse_account_extension, try_parse_mint_extension, AccountExtension, MintExtension,
        ParsedExtension,
    },
};

use super::interface::{TokenInterface, TokenState};
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{transfer_fee::TransferFeeConfig, ExtensionType, StateWithExtensions},
    state::{Account as Token2022AccountState, AccountState, Mint as Token2022MintState},
};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug)]
pub struct Token2022Account {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: u8,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
    // Extensions types present on the account (used for speed when we don't need the data of the actual extensions)
    pub extensions_types: Vec<ExtensionType>,
    /// Parsed extension data stored by extension type discriminant
    pub extensions: HashMap<u16, ParsedExtension>,
}

impl TokenState for Token2022Account {
    fn mint(&self) -> Pubkey {
        self.mint
    }
    fn owner(&self) -> Pubkey {
        self.owner
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Token2022Extensions for Token2022Account {
    fn get_extensions(&self) -> &HashMap<u16, ParsedExtension> {
        &self.extensions
    }

    fn get_extension_types(&self) -> &Vec<ExtensionType> {
        &self.extensions_types
    }
}

#[derive(Debug)]
pub struct Token2022Mint {
    pub mint: Pubkey,
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
    // Extensions types present on the mint (used for speed when we don't need the data of the actual extensions)
    pub extensions_types: Vec<ExtensionType>,
    /// Parsed extension data stored by extension type discriminant
    pub extensions: HashMap<u16, ParsedExtension>,
}

impl TokenMint for Token2022Mint {
    fn decimals(&self) -> u8 {
        self.decimals
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Token2022Mint {
    fn get_transfer_fee(&self) -> Option<TransferFeeConfig> {
        match self.get_extension(ExtensionType::TransferFeeConfig) {
            Some(ParsedExtension::Mint(MintExtension::TransferFeeConfig(config))) => Some(*config),
            _ => None,
        }
    }

    /// Calculate transfer fee for a given amount and epoch
    /// Returns None if no transfer fee is configured
    pub fn calculate_transfer_fee(
        &self,
        amount: u64,
        current_epoch: u64,
    ) -> Result<Option<u64>, crate::error::KoraError> {
        if let Some(fee_config) = self.get_transfer_fee() {
            let transfer_fee = if current_epoch >= u64::from(fee_config.newer_transfer_fee.epoch) {
                &fee_config.newer_transfer_fee
            } else {
                &fee_config.older_transfer_fee
            };

            let basis_points = u16::from(transfer_fee.transfer_fee_basis_points);
            let maximum_fee = u64::from(transfer_fee.maximum_fee);

            let fee_amount = (amount as u128)
                .checked_mul(basis_points as u128)
                .and_then(|product| product.checked_add(10_000 - 1))
                .and_then(|product| product.checked_div(10_000))
                .and_then(
                    |result| if result <= u64::MAX as u128 { Some(result as u64) } else { None },
                )
                .ok_or_else(|| {
                    log::error!(
                        "Transfer fee calculation overflow: amount={}, basis_points={}",
                        amount,
                        basis_points
                    );
                    crate::error::KoraError::ValidationError(format!(
                        "Transfer fee calculation overflow: amount={}, basis_points={}",
                        amount, basis_points
                    ))
                })?;
            Ok(Some(std::cmp::min(fee_amount, maximum_fee)))
        } else {
            Ok(None)
        }
    }
}

impl Token2022Extensions for Token2022Mint {
    fn get_extensions(&self) -> &HashMap<u16, ParsedExtension> {
        &self.extensions
    }

    fn get_extension_types(&self) -> &Vec<ExtensionType> {
        &self.extensions_types
    }
}

pub struct Token2022Program;

impl Token2022Program {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Token2022Program {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenInterface for Token2022Program {
    fn program_id(&self) -> Pubkey {
        spl_token_2022_interface::id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = StateWithExtensions::<Token2022AccountState>::unpack(data)?;
        let base = account.base;

        // Parse all extensions and store in HashMap
        let mut extensions = HashMap::new();
        let mut extensions_types = Vec::new();

        if data.len() > Token2022AccountState::LEN {
            for &extension_type in AccountExtension::EXTENSIONS {
                if let Some(parsed_ext) = try_parse_account_extension(&account, extension_type) {
                    extensions.insert(extension_type as u16, parsed_ext);
                    extensions_types.push(extension_type);
                }
            }
        }

        Ok(Box::new(Token2022Account {
            mint: base.mint,
            owner: base.owner,
            amount: base.amount,
            delegate: base.delegate.into(),
            state: match base.state {
                AccountState::Uninitialized => 0,
                AccountState::Initialized => 1,
                AccountState::Frozen => 2,
            },
            is_native: base.is_native.into(),
            delegated_amount: base.delegated_amount,
            close_authority: base.close_authority.into(),
            extensions_types,
            extensions,
        }))
    }

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let mint_with_extensions = StateWithExtensions::<Token2022MintState>::unpack(mint_data)?;
        let base = mint_with_extensions.base;

        // Parse all extensions and store in HashMap
        let mut extensions = HashMap::new();
        let mut extensions_types = Vec::new();

        if mint_data.len() > Token2022MintState::LEN {
            for &extension_type in MintExtension::EXTENSIONS {
                if let Some(parsed_ext) =
                    try_parse_mint_extension(&mint_with_extensions, extension_type)
                {
                    extensions.insert(extension_type as u16, parsed_ext);
                    extensions_types.push(extension_type);
                }
            }
        }

        Ok(Box::new(Token2022Mint {
            mint: *mint,
            mint_authority: base.mint_authority.into(),
            supply: base.supply,
            decimals: base.decimals,
            is_initialized: base.is_initialized,
            freeze_authority: base.freeze_authority.into(),
            extensions_types,
            extensions,
        }))
    }
}

/// Trait for Token-2022 extension validation and fee calculation
pub trait Token2022Extensions {
    /// Provide access to the extensions HashMap
    fn get_extensions(&self) -> &HashMap<u16, ParsedExtension>;

    /// Get all extension types
    fn get_extension_types(&self) -> &Vec<ExtensionType>;

    /// Helper function to convert ExtensionType to u16 key
    fn extension_key(ext_type: ExtensionType) -> u16 {
        ext_type as u16
    }

    /// Get extension by type
    fn get_extension(&self, extension_type: ExtensionType) -> Option<&ParsedExtension> {
        self.get_extensions().get(&Self::extension_key(extension_type))
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::common::{
        create_transfer_fee_config, MintAccountMockBuilder, TokenAccountMockBuilder,
    };

    use super::*;
    use solana_nullable::MaybeNull;
    use solana_sdk::pubkey::Pubkey;
    use spl_pod::primitives::{PodU16, PodU64};
    use spl_token_2022_interface::extension::{
        transfer_fee::{TransferFee, TransferFeeConfig},
        ExtensionType,
    };

    pub fn create_test_extensions() -> HashMap<u16, ParsedExtension> {
        let mut extensions = HashMap::new();
        extensions.insert(
            ExtensionType::TransferFeeConfig as u16,
            ParsedExtension::Mint(MintExtension::TransferFeeConfig(create_transfer_fee_config(
                100, 1000,
            ))),
        );
        extensions
    }

    #[test]
    fn test_token_program_token2022() {
        let program = Token2022Program::new();
        assert_eq!(program.program_id(), spl_token_2022_interface::id());
    }

    #[test]
    fn test_token2022_account_state() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a Token2022Account directly
        let account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extensions_types: vec![ExtensionType::TransferFeeConfig],
            extensions: create_test_extensions(),
        };

        // Verify the basic fields
        assert_eq!(account.mint(), mint);
        assert_eq!(account.owner(), owner);
        assert_eq!(account.amount, amount);

        // Verify extensions map is available
        assert!(!account.extensions.is_empty());
    }

    #[test]
    fn test_token2022_extension_support() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        let token_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&owner)
            .with_amount(amount)
            .build_as_custom_token2022_token_account(create_test_extensions());

        assert_eq!(token_account.mint(), mint);
        assert_eq!(token_account.owner(), owner);
        assert_eq!(token_account.amount, amount);

        assert!(!token_account.extensions.is_empty());
    }

    #[test]
    fn test_token2022_mint_transfer_fee_edge_cases() {
        let mint_pubkey = Pubkey::new_unique();

        let mint = MintAccountMockBuilder::new()
            .build_as_custom_token2022_mint(mint_pubkey, HashMap::new());

        let fee = mint.calculate_transfer_fee(1000, 0).unwrap();
        assert!(fee.is_none(), "Mint without transfer fee config should return None");

        let mint = MintAccountMockBuilder::new()
            .build_as_custom_token2022_mint(mint_pubkey, create_test_extensions());

        // Test zero amount
        let zero_fee = mint.calculate_transfer_fee(0, 0).unwrap();
        assert!(zero_fee.is_some());
        assert_eq!(zero_fee.unwrap(), 0, "Zero amount should result in zero fee");

        // Test maximum fee cap
        let large_amount_fee = mint.calculate_transfer_fee(1_000_000, 0).unwrap();
        assert!(large_amount_fee.is_some());
        assert_eq!(large_amount_fee.unwrap(), 1000, "Large amount should be capped at maximum fee");
    }

    #[test]
    fn test_token2022_mint_transfer_fee_calculation_with_fee() {
        let mint_pubkey = Pubkey::new_unique();
        let mut extensions = HashMap::new();
        extensions.insert(
            ExtensionType::TransferFeeConfig as u16,
            ParsedExtension::Mint(MintExtension::TransferFeeConfig(
                crate::tests::account_mock::create_transfer_fee_config(250, 1000),
            )), // 2.5%, max 1000
        );

        let mint = MintAccountMockBuilder::new()
            .with_extensions(vec![ExtensionType::TransferFeeConfig])
            .build_as_custom_token2022_mint(mint_pubkey, extensions);

        // Test fee calculation with transfer fee (ceiling division to match on-chain)
        let test_cases: Vec<(u64, u64)> = vec![
            (10000, 250),   // ceil(10000 * 250 / 10000) = 250
            (100000, 1000), // ceil(100000 * 250 / 10000) = 2500, capped at max 1000
            (1000, 25),     // ceil(1000 * 250 / 10000) = 25
            (101, 3),       // ceil(101 * 250 / 10000) = ceil(2.525) = 3
            (0, 0),         // Zero amount = zero fee
        ];

        for (amount, expected_fee) in test_cases {
            let fee = mint.calculate_transfer_fee(amount, 0).unwrap().unwrap_or(0);
            assert_eq!(
                fee, expected_fee,
                "Fee mismatch for amount={amount}: got={fee}, expected={expected_fee}"
            );
        }
    }

    #[test]
    fn test_token2022_mint_transfer_fee_epoch_handling() {
        let mint_pubkey = Pubkey::new_unique();

        // Create config with different fees for different epochs
        let transfer_fee_config = TransferFeeConfig {
            transfer_fee_config_authority: MaybeNull::try_from(Some(
                spl_pod::solana_pubkey::Pubkey::new_unique(),
            ))
            .unwrap(),
            withdraw_withheld_authority: MaybeNull::try_from(Some(
                spl_pod::solana_pubkey::Pubkey::new_unique(),
            ))
            .unwrap(),
            withheld_amount: PodU64::from(0),
            older_transfer_fee: TransferFee {
                epoch: PodU64::from(0),
                transfer_fee_basis_points: PodU16::from(100), // 1%
                maximum_fee: PodU64::from(500),
            },
            newer_transfer_fee: TransferFee {
                epoch: PodU64::from(10),
                transfer_fee_basis_points: PodU16::from(200), // 2%
                maximum_fee: PodU64::from(1000),
            },
        };

        let mut extensions = HashMap::new();
        extensions.insert(
            ExtensionType::TransferFeeConfig as u16,
            ParsedExtension::Mint(MintExtension::TransferFeeConfig(transfer_fee_config)),
        );

        let mint = MintAccountMockBuilder::new()
            .with_extensions(vec![ExtensionType::TransferFeeConfig])
            .build_as_custom_token2022_mint(mint_pubkey, extensions);

        // Test older fee (epoch < 10)
        let fee_old = mint.calculate_transfer_fee(10000, 5).unwrap().unwrap();
        assert_eq!(fee_old, 100); // 10000 * 1% = 100

        // Test newer fee (epoch >= 10)
        let fee_new = mint.calculate_transfer_fee(10000, 15).unwrap().unwrap();
        assert_eq!(fee_new, 200); // 10000 * 2% = 200
    }
}
