use std::collections::HashMap;

use solana_program::program_pack::Pack;
use solana_sdk::{account::Account, program_option::COption, pubkey::Pubkey};
use spl_pod::{
    optional_keys::OptionalNonZeroPubkey,
    primitives::{PodU16, PodU64},
};
use spl_token::state::{Account as TokenAccount, AccountState as SplAccountState, Mint};
use spl_token_2022::{
    extension::{
        self,
        transfer_fee::{TransferFee, TransferFeeConfig},
        BaseStateWithExtensionsMut, ExtensionType, PodStateWithExtensionsMut,
    },
    pod::PodMint,
    state::{
        Account as Token2022AccountState, AccountState as Token2022AccountState_, Mint as Mint2022,
    },
};

use crate::token::{
    spl_token_2022::Token2022Mint, spl_token_2022_util::ParsedExtension, Token2022Account,
};

// Common default values used across mock builders
const DEFAULT_LAMPORTS: u64 = 1_000_000;
const DEFAULT_TOKEN_AMOUNT: u64 = 100;
const DEFAULT_MINT_SUPPLY: u64 = 1_000_000_000_000;
const DEFAULT_RENT_EPOCH: u64 = 0;

fn into_rust_option<T>(c_option: COption<T>) -> Option<T> {
    match c_option {
        COption::Some(value) => Some(value),
        COption::None => None,
    }
}

pub struct AccountMockBuilder {
    lamports: u64,
    data: Vec<u8>,
    owner: Pubkey,
    executable: bool,
    rent_epoch: u64,
}

impl Default for AccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountMockBuilder {
    pub fn new() -> Self {
        Self {
            lamports: DEFAULT_LAMPORTS,
            data: vec![0u8; 100],
            owner: Pubkey::new_unique(),
            executable: false,
            rent_epoch: DEFAULT_RENT_EPOCH,
        }
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn with_owner(mut self, owner: Pubkey) -> Self {
        self.owner = owner;
        self
    }

    pub fn with_executable(mut self, executable: bool) -> Self {
        self.executable = executable;
        self
    }

    pub fn with_rent_epoch(mut self, rent_epoch: u64) -> Self {
        self.rent_epoch = rent_epoch;
        self
    }

    pub fn build(self) -> Account {
        Account {
            lamports: self.lamports,
            data: self.data,
            owner: self.owner,
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        }
    }
}

/// Unified token account builder supporting both SPL Token and Token2022
///
/// Use `build()` for SPL Token accounts or `build_token2022()` for Token2022 accounts
pub struct TokenAccountMockBuilder {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    delegate: COption<Pubkey>,
    delegated_amount: u64,
    close_authority: COption<Pubkey>,
    lamports: u64,
    rent_epoch: u64,
    // Token2022-specific fields
    extensions: Vec<ExtensionType>,
    // Configuration for different token types
    is_native_spl: COption<u64>,
    is_native_token2022: COption<u64>,
}

impl Default for TokenAccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenAccountMockBuilder {
    pub fn new() -> Self {
        Self {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: DEFAULT_TOKEN_AMOUNT,
            delegate: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
            lamports: DEFAULT_LAMPORTS,
            rent_epoch: DEFAULT_RENT_EPOCH,
            extensions: Vec::new(),
            is_native_spl: COption::Some(0),
            is_native_token2022: COption::None,
        }
    }

    pub fn with_mint(mut self, mint: &Pubkey) -> Self {
        self.mint = *mint;
        self
    }

    pub fn with_owner(mut self, owner: &Pubkey) -> Self {
        self.owner = *owner;
        self
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn with_delegate(mut self, delegate: Option<Pubkey>) -> Self {
        self.delegate = match delegate {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    /// Set native amount for SPL Token
    pub fn with_native_spl(mut self, native_amount: Option<u64>) -> Self {
        self.is_native_spl = match native_amount {
            Some(amount) => COption::Some(amount),
            None => COption::None,
        };
        self
    }

    /// Set native amount for Token2022
    pub fn with_native_token2022(mut self, native_amount: Option<u64>) -> Self {
        self.is_native_token2022 = match native_amount {
            Some(amount) => COption::Some(amount),
            None => COption::None,
        };
        self
    }

    /// Add an extension type (Token2022 only)
    pub fn with_extension(mut self, extension: ExtensionType) -> Self {
        if !self.extensions.contains(&extension) {
            self.extensions.push(extension);
        }
        self
    }

    /// Add multiple extension types (Token2022 only)
    pub fn with_extensions(mut self, extensions: Vec<ExtensionType>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set account state (for SPL Token compatibility)
    pub fn with_state(self, _state: SplAccountState) -> Self {
        // State is handled automatically in build methods, this is for compatibility
        self
    }

    /// Legacy method for backward compatibility (defaults to SPL Token behavior)
    pub fn with_native(self, native_amount: Option<u64>) -> Self {
        self.with_native_spl(native_amount)
    }

    pub fn with_delegated_amount(mut self, amount: u64) -> Self {
        self.delegated_amount = amount;
        self
    }

    pub fn with_close_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.close_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    /// Build SPL Token account
    pub fn build(self) -> Account {
        let token_account = TokenAccount {
            mint: self.mint,
            owner: self.owner,
            amount: self.amount,
            delegate: self.delegate,
            state: SplAccountState::Initialized,
            is_native: self.is_native_spl,
            delegated_amount: self.delegated_amount,
            close_authority: self.close_authority,
        };

        let mut data = vec![0u8; TokenAccount::LEN];
        token_account.pack_into_slice(&mut data);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }

    /// Build Token2022 account
    pub fn build_token2022(self) -> Account {
        let base_account = Token2022AccountState {
            mint: self.mint,
            owner: self.owner,
            amount: self.amount,
            delegate: self.delegate,
            state: Token2022AccountState_::Initialized,
            is_native: self.is_native_token2022,
            delegated_amount: self.delegated_amount,
            close_authority: self.close_authority,
        };

        let mut data = vec![0u8; Token2022AccountState::LEN];
        base_account.pack_into_slice(&mut data[..Token2022AccountState::LEN]);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token_2022::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }

    /// Build Token2022 account as custom structure (for Token2022Account type)
    pub fn build_as_custom_token2022_token_account(
        self,
        extensions: HashMap<u16, ParsedExtension>,
    ) -> Token2022Account {
        Token2022Account {
            mint: self.mint,
            owner: self.owner,
            amount: self.amount,
            delegate: into_rust_option(self.delegate),
            state: Token2022AccountState_::Initialized.into(),
            is_native: into_rust_option(self.is_native_token2022),
            delegated_amount: self.delegated_amount,
            close_authority: into_rust_option(self.close_authority),
            extensions_types: self.extensions.clone(),
            extensions,
        }
    }
}

/// Unified mint account builder supporting both SPL Token and Token2022
///
/// Use `build()` for SPL Token mints or `build_token2022()` for Token2022 mints
pub struct MintAccountMockBuilder {
    mint_authority: COption<Pubkey>,
    supply: u64,
    decimals: u8,
    is_initialized: bool,
    freeze_authority: COption<Pubkey>,
    lamports: u64,
    rent_epoch: u64,
    // Token2022-specific fields
    extensions: Vec<ExtensionType>,
}

impl Default for MintAccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MintAccountMockBuilder {
    pub fn new() -> Self {
        Self {
            mint_authority: COption::Some(Pubkey::new_unique()),
            supply: DEFAULT_MINT_SUPPLY,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
            lamports: 0,
            rent_epoch: DEFAULT_RENT_EPOCH,
            extensions: Vec::new(),
        }
    }

    pub fn with_mint_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.mint_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_supply(mut self, supply: u64) -> Self {
        self.supply = supply;
        self
    }

    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn with_initialized(mut self, initialized: bool) -> Self {
        self.is_initialized = initialized;
        self
    }

    pub fn with_freeze_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.freeze_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    /// Add an extension type (Token2022 only)
    pub fn with_extension(mut self, extension: ExtensionType) -> Self {
        if !self.extensions.contains(&extension) {
            self.extensions.push(extension);
        }
        self
    }

    /// Add multiple extension types (Token2022 only)
    pub fn with_extensions(mut self, extensions: Vec<ExtensionType>) -> Self {
        for ext in extensions {
            if !self.extensions.contains(&ext) {
                self.extensions.push(ext);
            }
        }
        self
    }

    pub fn build(self) -> Account {
        let mint_data = Mint {
            mint_authority: self.mint_authority,
            supply: self.supply,
            decimals: self.decimals,
            is_initialized: self.is_initialized,
            freeze_authority: self.freeze_authority,
        };

        let mut data = vec![0u8; Mint::LEN];
        mint_data.pack_into_slice(&mut data);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }

    pub fn build_token2022(self) -> Account {
        if !self.extensions.is_empty() {
            self.build_token2022_account_state_with_extensions().unwrap()
        } else {
            let base_mint = Mint2022 {
                mint_authority: self.mint_authority,
                supply: self.supply,
                decimals: self.decimals,
                is_initialized: self.is_initialized,
                freeze_authority: self.freeze_authority,
            };

            let mut data = vec![0u8; Mint2022::LEN];
            base_mint.pack_into_slice(&mut data[..Mint2022::LEN]);

            Account {
                lamports: self.lamports,
                data,
                owner: spl_token_2022::id(),
                executable: false,
                rent_epoch: self.rent_epoch,
            }
        }
    }

    /// Build Token2022 mint with extensions (returns Result<Account>)
    pub fn build_token2022_account_state_with_extensions(
        self,
    ) -> Result<Account, Box<dyn std::error::Error>> {
        let account_len = if !self.extensions.is_empty() {
            ExtensionType::try_calculate_account_len::<Mint2022>(&self.extensions).unwrap()
        } else {
            Mint2022::LEN
        };

        let mut data = vec![0u8; account_len];

        let base_mint = Mint2022 {
            mint_authority: self.mint_authority,
            supply: self.supply,
            decimals: self.decimals,
            is_initialized: self.is_initialized,
            freeze_authority: self.freeze_authority,
        };

        if self.extensions.is_empty() {
            base_mint.pack_into_slice(&mut data);
        } else {
            let mut state = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut data)?;

            // Initialize each extension
            for extension_type in &self.extensions {
                match extension_type {
                    ExtensionType::MintCloseAuthority => {
                        state
                            .init_extension::<extension::mint_close_authority::MintCloseAuthority>(
                                true,
                            )?;
                    }
                    ExtensionType::TransferFeeConfig => {
                        state.init_extension::<extension::transfer_fee::TransferFeeConfig>(true)?;
                    }
                    // Add other extension types as needed
                    _ => {}
                }
            }

            let pod_mint = PodMint {
                mint_authority: base_mint.mint_authority.into(),
                supply: base_mint.supply.into(),
                decimals: base_mint.decimals,
                is_initialized: base_mint.is_initialized.into(),
                freeze_authority: base_mint.freeze_authority.into(),
            };
            *state.base = pod_mint;
            state.init_account_type()?;
        }

        Ok(Account {
            lamports: self.lamports,
            data,
            owner: spl_token_2022::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        })
    }

    /// Build Token2022 mint as custom structure (for Token2022Mint type)
    pub fn build_as_custom_token2022_mint(
        self,
        mint_pubkey: Pubkey,
        extensions: HashMap<u16, ParsedExtension>,
    ) -> Token2022Mint {
        Token2022Mint {
            mint: mint_pubkey,
            mint_authority: into_rust_option(self.mint_authority),
            supply: self.supply,
            decimals: self.decimals,
            is_initialized: self.is_initialized,
            freeze_authority: into_rust_option(self.freeze_authority),
            extensions_types: self.extensions.clone(),
            extensions,
        }
    }
}

// Helper functions for test account creation

pub fn create_mock_account() -> Account {
    AccountMockBuilder::new().build()
}

pub fn create_mock_program_account() -> Account {
    AccountMockBuilder::new()
        .with_executable(true)
        .with_owner(Pubkey::new_unique())
        .with_data(vec![0u8; 100])
        .build()
}

pub fn create_mock_non_executable_account() -> Account {
    AccountMockBuilder::new().with_executable(false).build()
}

pub fn create_mock_token_account(owner: &Pubkey, mint: &Pubkey) -> Account {
    TokenAccountMockBuilder::new().with_owner(owner).with_mint(mint).build()
}

pub fn create_mock_spl_mint_account(decimals: u8) -> Account {
    MintAccountMockBuilder::new().with_decimals(decimals).build()
}

pub fn create_mock_token2022_mint_account(decimals: u8) -> Account {
    MintAccountMockBuilder::new().with_decimals(decimals).build_token2022()
}

pub fn create_mock_account_with_balance(lamports: u64) -> Account {
    AccountMockBuilder::new().with_lamports(lamports).build()
}

pub fn create_mock_account_with_owner(owner: Pubkey) -> Account {
    AccountMockBuilder::new().with_owner(owner).build()
}

pub fn create_mock_usdc_mint_account() -> Account {
    MintAccountMockBuilder::new()
        .with_decimals(6)
        .with_supply(DEFAULT_MINT_SUPPLY) // 1M USDC with 6 decimals
        .build()
}

/// Create mock SOL wrapped token mint (9 decimals)
pub fn create_mock_wsol_mint_account() -> Account {
    MintAccountMockBuilder::new().with_decimals(9).build()
}

/// Create mock Token2022 account with specific extensions
pub fn create_mock_token2022_account_with_extensions(
    owner: &Pubkey,
    mint: &Pubkey,
    extensions: Vec<ExtensionType>,
) -> Account {
    TokenAccountMockBuilder::new()
        .with_owner(owner)
        .with_mint(mint)
        .with_extensions(extensions)
        .build_token2022()
}

/// Create mock Token2022 mint with specific extensions
pub fn create_mock_token2022_mint_with_extensions(
    decimals: u8,
    extensions: Vec<ExtensionType>,
) -> Account {
    MintAccountMockBuilder::new()
        .with_decimals(decimals)
        .with_extensions(extensions)
        .build_token2022()
}

// ========== Token2022 Test Helpers ==========

/// Helper to create Transfer Fee Config for testing
pub fn create_transfer_fee_config(basis_points: u16, max_fee: u64) -> TransferFeeConfig {
    TransferFeeConfig {
        transfer_fee_config_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique()))
            .unwrap(),
        withdraw_withheld_authority: OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique()))
            .unwrap(),
        withheld_amount: PodU64::from(0),
        newer_transfer_fee: TransferFee {
            epoch: PodU64::from(0),
            transfer_fee_basis_points: PodU16::from(basis_points),
            maximum_fee: PodU64::from(max_fee),
        },
        older_transfer_fee: TransferFee {
            epoch: PodU64::from(0),
            transfer_fee_basis_points: PodU16::from(0),
            maximum_fee: PodU64::from(0),
        },
    }
}
