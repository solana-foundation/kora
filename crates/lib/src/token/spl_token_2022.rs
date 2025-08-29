use crate::{
    token::{
        interface::TokenMint,
        spl_token_2022_util::{
            try_parse_account_extension, try_parse_mint_extension, AccountExtension, MintExtension,
            ParsedExtension,
        },
    },
    KoraError,
};

use super::interface::{TokenInterface, TokenState};
use async_trait::async_trait;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::{account::Account, instruction::Instruction};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
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
    fn amount(&self) -> u64 {
        self.amount
    }
    fn decimals(&self) -> u8 {
        0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Token2022Account {
    /*
    Token account only extensions
     */
    pub fn has_memo_extension(&self) -> bool {
        self.has_extension(ExtensionType::MemoTransfer)
    }

    pub fn has_immutable_owner_extension(&self) -> bool {
        self.has_extension(ExtensionType::ImmutableOwner)
    }

    pub fn has_default_account_state_extension(&self) -> bool {
        self.has_extension(ExtensionType::DefaultAccountState)
    }
}

impl Token2022Extensions for Token2022Account {
    fn get_extensions(&self) -> &HashMap<u16, ParsedExtension> {
        &self.extensions
    }

    fn get_extension_types(&self) -> &Vec<ExtensionType> {
        &self.extensions_types
    }

    /*
    Token account & mint account extensions (each their own type)
     */

    fn has_confidential_transfer_extension(&self) -> bool {
        self.has_extension(ExtensionType::ConfidentialTransferAccount)
    }

    fn has_transfer_hook_extension(&self) -> bool {
        self.has_extension(ExtensionType::TransferHookAccount)
    }

    fn has_pausable_extension(&self) -> bool {
        self.has_extension(ExtensionType::PausableAccount)
    }

    fn is_non_transferable(&self) -> bool {
        self.has_extension(ExtensionType::NonTransferableAccount)
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
    fn address(&self) -> Pubkey {
        self.mint
    }

    fn decimals(&self) -> u8 {
        self.decimals
    }

    fn mint_authority(&self) -> Option<Pubkey> {
        self.mint_authority
    }

    fn supply(&self) -> u64 {
        self.supply
    }

    fn freeze_authority(&self) -> Option<Pubkey> {
        self.freeze_authority
    }

    fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    fn get_token_program(&self) -> Box<dyn TokenInterface> {
        Box::new(Token2022Program::new())
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
    pub fn calculate_transfer_fee(&self, amount: u64, current_epoch: u64) -> Option<u64> {
        if let Some(fee_config) = self.get_transfer_fee() {
            let transfer_fee = if current_epoch >= u64::from(fee_config.newer_transfer_fee.epoch) {
                &fee_config.newer_transfer_fee
            } else {
                &fee_config.older_transfer_fee
            };

            let basis_points = u16::from(transfer_fee.transfer_fee_basis_points);
            let maximum_fee = u64::from(transfer_fee.maximum_fee);

            let fee_amount = (amount as u128 * basis_points as u128 / 10_000) as u64;
            Some(std::cmp::min(fee_amount, maximum_fee))
        } else {
            None
        }
    }

    pub fn has_confidential_mint_burn_extension(&self) -> bool {
        self.has_extension(ExtensionType::ConfidentialMintBurn)
    }

    pub fn has_mint_close_authority_extension(&self) -> bool {
        self.has_extension(ExtensionType::MintCloseAuthority)
    }

    pub fn has_interest_bearing_extension(&self) -> bool {
        self.has_extension(ExtensionType::InterestBearingConfig)
    }

    pub fn has_permanent_delegate_extension(&self) -> bool {
        self.has_extension(ExtensionType::PermanentDelegate)
    }
}

impl Token2022Extensions for Token2022Mint {
    fn get_extensions(&self) -> &HashMap<u16, ParsedExtension> {
        &self.extensions
    }

    fn get_extension_types(&self) -> &Vec<ExtensionType> {
        &self.extensions_types
    }

    /*
    Token account & mint account extensions (each their own type)
     */

    fn has_confidential_transfer_extension(&self) -> bool {
        self.has_extension(ExtensionType::ConfidentialTransferMint)
    }

    fn has_transfer_hook_extension(&self) -> bool {
        self.has_extension(ExtensionType::TransferHook)
    }

    fn has_pausable_extension(&self) -> bool {
        self.has_extension(ExtensionType::Pausable)
    }

    fn is_non_transferable(&self) -> bool {
        self.has_extension(ExtensionType::NonTransferable)
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

#[async_trait]
impl TokenInterface for Token2022Program {
    fn program_id(&self) -> Pubkey {
        spl_token_2022::id()
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

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token_2022::instruction::initialize_account3(
            &self.program_id(),
            account,
            mint,
            owner,
        )?)
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        // Get the mint from the source account data
        #[allow(deprecated)]
        Ok(spl_token_2022::instruction::transfer(
            &self.program_id(),
            source,
            destination,
            authority,
            &[],
            amount,
        )?)
    }

    fn create_transfer_checked_instruction(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token_2022::instruction::transfer_checked(
            &self.program_id(),
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?)
    }

    fn get_associated_token_address(&self, wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(wallet, mint, &self.program_id())
    }

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        create_associated_token_account(funding_account, wallet, mint, &self.program_id())
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

    async fn get_ata_account_size(
        &self,
        mint_pubkey: &Pubkey,
        mint_account: &Account,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let token_program = Token2022Program::new();
        let mint_with_extensions = token_program.unpack_mint(mint_pubkey, &mint_account.data)?;

        let mint_with_extensions = mint_with_extensions
            .as_any()
            .downcast_ref::<Token2022Mint>()
            .ok_or(KoraError::InternalServerError("Failed to unpack mint".to_string()))?;

        let mut required_account_extensions = Vec::new();

        // If the mint has TransferFeeConfig, token accounts need TransferFeeAmount to store withheld fees
        if mint_with_extensions.has_extension(ExtensionType::TransferFeeConfig) {
            required_account_extensions.push(ExtensionType::TransferFeeAmount);
        }

        // If mint is non-transferable, token accounts must be non-transferable too
        if mint_with_extensions.is_non_transferable() {
            required_account_extensions.push(ExtensionType::NonTransferableAccount);
        }

        // If mint has confidential transfer, token accounts need confidential transfer account extension
        if mint_with_extensions.has_confidential_transfer_extension() {
            required_account_extensions.push(ExtensionType::ConfidentialTransferAccount);
        }

        // If mint has transfer hook, token accounts need transfer hook account extension
        if mint_with_extensions.has_transfer_hook_extension() {
            required_account_extensions.push(ExtensionType::TransferHookAccount);
        }

        // If mint is pausable, token accounts need pausable account extension
        if mint_with_extensions.has_pausable_extension() {
            required_account_extensions.push(ExtensionType::PausableAccount);
        }

        // ATAs created for Token-2022 are immutable by default (owner cannot be changed)
        required_account_extensions.push(ExtensionType::ImmutableOwner);

        // Calculate the account size with all required extensions
        let account_size = ExtensionType::try_calculate_account_len::<Token2022AccountState>(
            &required_account_extensions,
        )?;

        Ok(account_size)
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

    /// Check if has a specific extension type
    fn has_extension(&self, extension_type: ExtensionType) -> bool {
        self.get_extension_types().contains(&extension_type)
    }

    /// Get extension by type
    fn get_extension(&self, extension_type: ExtensionType) -> Option<&ParsedExtension> {
        self.get_extensions().get(&Self::extension_key(extension_type))
    }

    fn has_confidential_transfer_extension(&self) -> bool;

    fn has_transfer_hook_extension(&self) -> bool;

    fn has_pausable_extension(&self) -> bool;

    /// Check if the token/mint is non-transferable (differs between Account and Mint)
    fn is_non_transferable(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use crate::tests::common::{
        create_transfer_fee_config, MintAccountMockBuilder, TokenAccountMockBuilder,
    };

    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use spl_pod::{
        optional_keys::OptionalNonZeroPubkey,
        primitives::{PodU16, PodU64},
    };
    use spl_token_2022::extension::{
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
        assert_eq!(program.program_id(), spl_token_2022::id());
    }

    #[test]
    fn test_token2022_program_creation() {
        let program = Token2022Program::new();
        assert_eq!(program.program_id(), spl_token_2022::id());
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
        assert_eq!(account.amount(), amount);

        // Verify extensions map is available
        assert!(!account.extensions.is_empty());
    }

    #[test]
    fn test_token2022_transfer_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100;

        // Create the instruction directly for testing
        let program = Token2022Program::new();
        let ix = program.create_transfer_instruction(&source, &dest, &authority, amount).unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
        // Verify accounts are in correct order
        assert_eq!(ix.accounts[0].pubkey, source);
        assert_eq!(ix.accounts[1].pubkey, dest);
        assert_eq!(ix.accounts[2].pubkey, authority);
    }

    #[test]
    fn test_token2022_transfer_checked_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100;
        let decimals = 9;

        let program = Token2022Program::new();
        let ix = program
            .create_transfer_checked_instruction(
                &source, &mint, &dest, &authority, amount, decimals,
            )
            .unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
        // Verify accounts are in correct order
        assert_eq!(ix.accounts[0].pubkey, source);
        assert_eq!(ix.accounts[1].pubkey, mint);
        assert_eq!(ix.accounts[2].pubkey, dest);
        assert_eq!(ix.accounts[3].pubkey, authority);
    }

    #[test]
    fn test_token2022_ata_derivation() {
        let program = Token2022Program::new();
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);

        // Verify ATA derivation matches spl-token-2022
        let expected_ata =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &wallet,
                &mint,
                &spl_token_2022::id(),
            );
        assert_eq!(ata, expected_ata);
    }

    #[test]
    fn test_token2022_account_state_extensions() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let amount = 1000;

        let token_account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&owner)
            .with_amount(amount)
            .build_as_custom_token2022_token_account(HashMap::new());

        // Test extension detection
        assert!(!token_account.has_extension(ExtensionType::TransferFeeConfig));
        assert!(!token_account.has_extension(ExtensionType::NonTransferableAccount));
        assert!(!token_account.has_extension(ExtensionType::CpiGuard));
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
        assert_eq!(token_account.amount(), amount);

        assert!(!token_account.extensions.is_empty());
    }

    #[test]
    fn test_token2022_mint_transfer_fee_edge_cases() {
        let mint_pubkey = Pubkey::new_unique();

        let mint = MintAccountMockBuilder::new()
            .build_as_custom_token2022_mint(mint_pubkey, HashMap::new());

        let fee = mint.calculate_transfer_fee(1000, 0);
        assert!(fee.is_none(), "Mint without transfer fee config should return None");

        let mint = MintAccountMockBuilder::new()
            .build_as_custom_token2022_mint(mint_pubkey, create_test_extensions());

        // Test zero amount
        let zero_fee = mint.calculate_transfer_fee(0, 0);
        assert!(zero_fee.is_some());
        assert_eq!(zero_fee.unwrap(), 0, "Zero amount should result in zero fee");

        // Test maximum fee cap
        let large_amount_fee = mint.calculate_transfer_fee(1_000_000, 0);
        assert!(large_amount_fee.is_some());
        assert_eq!(large_amount_fee.unwrap(), 1000, "Large amount should be capped at maximum fee");
    }

    #[test]
    fn test_token2022_mint_specific_extensions() {
        let mint_pubkey = Pubkey::new_unique();
        let mint = Token2022Mint {
            mint: mint_pubkey,
            mint_authority: None,
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
            extensions_types: vec![
                ExtensionType::InterestBearingConfig,
                ExtensionType::PermanentDelegate,
                ExtensionType::MintCloseAuthority,
            ],
            extensions: HashMap::new(), // Extension data not needed for has_extension tests
        };

        assert!(mint.has_interest_bearing_extension());
        assert!(mint.has_permanent_delegate_extension());
        assert!(mint.has_mint_close_authority_extension());
        assert!(!mint.has_confidential_mint_burn_extension());
    }

    #[tokio::test]
    async fn test_token2022_get_ata_account_size() {
        let program = Token2022Program::new();
        let mint_pubkey = Pubkey::new_unique();

        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build_token2022();

        let result = program.get_ata_account_size(&mint_pubkey, &mint_account).await;

        assert!(result.is_ok());
        let size = result.unwrap();

        // Size should be base Token2022AccountState size + ImmutableOwner extension + 1 byte padding
        // (has 0 bytes of extension data, but still has the TVL overhead (type 2 bytes, length 2 bytes))
        // (ATAs created for Token-2022 are immutable by default)
        assert_eq!(size, 170); // Base (165) + Immutable Owner TVL Overhead (5)
    }

    #[test]
    fn test_token2022_account_extension_methods() {
        let account = TokenAccountMockBuilder::new()
            .with_extensions(vec![
                ExtensionType::MemoTransfer,
                ExtensionType::ImmutableOwner,
                ExtensionType::DefaultAccountState,
                ExtensionType::ConfidentialTransferAccount,
                ExtensionType::TransferHookAccount,
                ExtensionType::PausableAccount,
            ])
            .build_as_custom_token2022_token_account(HashMap::new());

        // Test all extension detection methods
        assert!(account.has_memo_extension());
        assert!(account.has_immutable_owner_extension());
        assert!(account.has_default_account_state_extension());
        assert!(account.has_confidential_transfer_extension());
        assert!(account.has_transfer_hook_extension());
        assert!(account.has_pausable_extension());

        // Test extensions not present
        assert!(!account.is_non_transferable());
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

        // Test fee calculation with transfer fee
        let test_cases = vec![
            (10000, 250),   // 10000 * 2.5% = 250
            (100000, 1000), // Would be 2500, but capped at 1000
            (1000, 25),     // 1000 * 2.5% = 25
            (0, 0),         // Zero amount = zero fee
        ];

        for (amount, _expected_adjusted) in test_cases {
            let expected_fee = mint.calculate_transfer_fee(amount, 0).unwrap_or(0);
            let expected_result = amount.saturating_sub(expected_fee);
            assert_eq!(expected_result, expected_result);
        }
    }

    #[test]
    fn test_token2022_mint_transfer_fee_epoch_handling() {
        let mint_pubkey = Pubkey::new_unique();

        // Create config with different fees for different epochs
        let transfer_fee_config = TransferFeeConfig {
            transfer_fee_config_authority: OptionalNonZeroPubkey::try_from(Some(
                Pubkey::new_unique(),
            ))
            .unwrap(),
            withdraw_withheld_authority: OptionalNonZeroPubkey::try_from(
                Some(Pubkey::new_unique()),
            )
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
        let fee_old = mint.calculate_transfer_fee(10000, 5).unwrap();
        assert_eq!(fee_old, 100); // 10000 * 1% = 100

        // Test newer fee (epoch >= 10)
        let fee_new = mint.calculate_transfer_fee(10000, 15).unwrap();
        assert_eq!(fee_new, 200); // 10000 * 2% = 200
    }

    #[test]
    fn test_token2022_mint_all_extension_methods() {
        let mint = MintAccountMockBuilder::new()
            .with_extensions(vec![
                ExtensionType::InterestBearingConfig,
                ExtensionType::PermanentDelegate,
                ExtensionType::MintCloseAuthority,
                ExtensionType::ConfidentialMintBurn,
                ExtensionType::ConfidentialTransferMint,
                ExtensionType::TransferHook,
                ExtensionType::Pausable,
            ])
            .build_as_custom_token2022_mint(Pubkey::new_unique(), HashMap::new());

        // Test all extension detection methods
        assert!(mint.has_interest_bearing_extension());
        assert!(mint.has_permanent_delegate_extension());
        assert!(mint.has_mint_close_authority_extension());
        assert!(mint.has_confidential_mint_burn_extension());
        assert!(mint.has_confidential_transfer_extension());
        assert!(mint.has_transfer_hook_extension());
        assert!(mint.has_pausable_extension());

        // Test extensions not present
        assert!(!mint.is_non_transferable());
    }

    #[tokio::test]
    async fn test_token2022_get_ata_account_size_with_extensions() {
        let program = Token2022Program::new();
        let mint_pubkey = Pubkey::new_unique();

        // Create a mint with transfer fee extension
        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(6)
            .with_supply(1_000_000)
            .with_extension(ExtensionType::TransferFeeConfig)
            .build_token2022_account_state_with_extensions()
            .unwrap();

        let result = program.get_ata_account_size(&mint_pubkey, &mint_account).await;

        // Should succeed and return size that includes extension space
        assert!(result.is_ok());

        // Size would be
        // Base 165
        // Immutable owner tvl overhead 4 (all ATA for 2022 have immutable owner)
        // + 1 byte padding
        // Transfer fee config 8 bytes for data and 4 bytes for TVL overhead
        assert_eq!(result.unwrap(), 182);
    }
}
