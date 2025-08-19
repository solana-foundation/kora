//! SPL Token 2022 extension utilities and parsing
//!
//! This module provides utilities for working with SPL Token 2022 extensions,
//! including parsing extension data and converting between string names and extension types.
//!
//! ## Macro-Generated Methods
//!
//! ### `define_extensions!` Macro
//! The `define_extensions!` macro generates extension enums with parsing methods:
//!
//! ```rust,ignore
//! // For MintExtension:
//! MintExtension::from_string("transfer_fee_config") -> Some(ExtensionType::TransferFeeConfig)
//! MintExtension::to_string_name(ExtensionType::TransferFeeConfig) -> Some("transfer_fee_config")  
//! MintExtension::all_string_names() -> &["confidential_transfer_mint", "transfer_fee_config", ...]
//! MintExtension::EXTENSIONS -> &[ExtensionType::ConfidentialTransferMint, ExtensionType::TransferFeeConfig, ...]
//! ```
//!
//! ## Utility Functions
//!
//! ### Extension Parsing Functions
//! The module also provides utility functions for parsing extensions from on-chain data:
//!
//! ```rust,ignore
//! // Parse extensions from mint/account state:
//! try_parse_mint_extension(&mint_state, ExtensionType::TransferFeeConfig) -> Option<ParsedExtension>
//! try_parse_account_extension(&account_state, ExtensionType::MemoTransfer) -> Option<ParsedExtension>
//!
//! // String-to-ExtensionType parsing:
//! parse_mint_extension_string("transfer_fee_config") -> Some(ExtensionType::TransferFeeConfig)
//! parse_account_extension_string("memo_transfer") -> Some(ExtensionType::MemoTransfer)
//!
//! // Get all valid extension names:
//! get_all_mint_extension_names() -> &["confidential_transfer_mint", "transfer_fee_config", ...]
//! get_all_account_extension_names() -> &["memo_transfer", "cpi_guard", ...]
//! ```

use spl_token_2022::{
    extension::{
        confidential_mint_burn::ConfidentialMintBurn,
        confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
        cpi_guard::CpiGuard,
        default_account_state::DefaultAccountState,
        immutable_owner::ImmutableOwner,
        interest_bearing_mint::InterestBearingConfig,
        memo_transfer::MemoTransfer,
        mint_close_authority::MintCloseAuthority,
        non_transferable::{NonTransferable, NonTransferableAccount},
        pausable::{PausableAccount, PausableConfig},
        permanent_delegate::PermanentDelegate,
        transfer_fee::TransferFeeConfig,
        transfer_hook::{TransferHook, TransferHookAccount},
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    state::{Account as Token2022AccountState, Mint as Token2022MintState},
};

macro_rules! define_extensions {
    ($name:ident, [$($variant:ident($type:ty) => $ext_type:path, $str_name:literal),* $(,)?]) => {
        #[derive(Debug, Clone)]
        pub enum $name {
            $($variant($type),)*
        }

        impl $name {
            pub const EXTENSIONS: &'static [ExtensionType] = &[$($ext_type,)*];
        }

        impl $name {
            pub fn from_string(s: &str) -> Option<ExtensionType> {
                match s {
                    $($str_name => Some($ext_type),)*
                    _ => None,
                }
            }

            pub fn to_string_name(ext_type: ExtensionType) -> Option<&'static str> {
                match ext_type {
                    $($ext_type => Some($str_name),)*
                    _ => None,
                }
            }

            pub fn all_string_names() -> &'static [&'static str] {
                &[$($str_name,)*]
            }
        }
    };
}

define_extensions!(MintExtension, [
    ConfidentialTransferConfig(ConfidentialTransferMint) => ExtensionType::ConfidentialTransferMint, "confidential_transfer_mint",
    ConfidentialMintBurn(ConfidentialMintBurn) => ExtensionType::ConfidentialMintBurn, "confidential_mint_burn",
    TransferFeeConfig(TransferFeeConfig) => ExtensionType::TransferFeeConfig, "transfer_fee_config",
    MintCloseAuthority(MintCloseAuthority) => ExtensionType::MintCloseAuthority, "mint_close_authority",
    InterestBearingConfig(InterestBearingConfig) => ExtensionType::InterestBearingConfig, "interest_bearing_config",
    NonTransferable(NonTransferable) => ExtensionType::NonTransferable, "non_transferable",
    PermanentDelegate(PermanentDelegate) => ExtensionType::PermanentDelegate, "permanent_delegate",
    TransferHook(TransferHook) => ExtensionType::TransferHook, "transfer_hook",
    PausableConfig(PausableConfig) => ExtensionType::Pausable, "pausable",
]);

define_extensions!(AccountExtension, [
    ConfidentialTransferAccount(Box<ConfidentialTransferAccount>) => ExtensionType::ConfidentialTransferAccount, "confidential_transfer_account",
    NonTransferableAccount(NonTransferableAccount) => ExtensionType::NonTransferableAccount, "non_transferable_account",
    TransferHook(TransferHookAccount) => ExtensionType::TransferHookAccount, "transfer_hook_account",
    PausableAccount(PausableAccount) => ExtensionType::PausableAccount, "pausable_account",
    MemoTransfer(MemoTransfer) => ExtensionType::MemoTransfer, "memo_transfer",
    CpiGuard(CpiGuard) => ExtensionType::CpiGuard, "cpi_guard",
    ImmutableOwner(ImmutableOwner) => ExtensionType::ImmutableOwner, "immutable_owner",
    DefaultAccountState(DefaultAccountState) => ExtensionType::DefaultAccountState, "default_account_state",
]);

#[derive(Debug, Clone)]
pub enum ParsedExtension {
    Mint(MintExtension),
    Account(AccountExtension),
}

pub fn try_parse_account_extension(
    account: &StateWithExtensions<Token2022AccountState>,
    ext_type: ExtensionType,
) -> Option<ParsedExtension> {
    match ext_type {
        ExtensionType::ConfidentialTransferAccount => {
            account.get_extension::<ConfidentialTransferAccount>().ok().map(|ext| {
                ParsedExtension::Account(AccountExtension::ConfidentialTransferAccount(Box::new(
                    *ext,
                )))
            })
        }
        ExtensionType::NonTransferableAccount => account
            .get_extension::<NonTransferableAccount>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::NonTransferableAccount(*ext))),
        ExtensionType::TransferHookAccount => account
            .get_extension::<TransferHookAccount>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::TransferHook(*ext))),
        ExtensionType::PausableAccount => account
            .get_extension::<PausableAccount>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::PausableAccount(*ext))),
        ExtensionType::MemoTransfer => account
            .get_extension::<MemoTransfer>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::MemoTransfer(*ext))),
        ExtensionType::CpiGuard => account
            .get_extension::<CpiGuard>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::CpiGuard(*ext))),
        ExtensionType::ImmutableOwner => account
            .get_extension::<ImmutableOwner>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::ImmutableOwner(*ext))),
        ExtensionType::DefaultAccountState => account
            .get_extension::<DefaultAccountState>()
            .ok()
            .map(|ext| ParsedExtension::Account(AccountExtension::DefaultAccountState(*ext))),
        _ => None,
    }
}

pub fn try_parse_mint_extension(
    mint: &StateWithExtensions<Token2022MintState>,
    ext_type: ExtensionType,
) -> Option<ParsedExtension> {
    match ext_type {
        ExtensionType::ConfidentialTransferMint => mint
            .get_extension::<ConfidentialTransferMint>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::ConfidentialTransferConfig(*ext))),
        ExtensionType::ConfidentialMintBurn => mint
            .get_extension::<ConfidentialMintBurn>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::ConfidentialMintBurn(*ext))),
        ExtensionType::TransferFeeConfig => mint
            .get_extension::<TransferFeeConfig>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::TransferFeeConfig(*ext))),
        ExtensionType::MintCloseAuthority => mint
            .get_extension::<MintCloseAuthority>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::MintCloseAuthority(*ext))),
        ExtensionType::InterestBearingConfig => mint
            .get_extension::<InterestBearingConfig>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::InterestBearingConfig(*ext))),
        ExtensionType::NonTransferable => mint
            .get_extension::<NonTransferable>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::NonTransferable(*ext))),
        ExtensionType::PermanentDelegate => mint
            .get_extension::<PermanentDelegate>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::PermanentDelegate(*ext))),
        ExtensionType::TransferHook => mint
            .get_extension::<TransferHook>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::TransferHook(*ext))),
        ExtensionType::Pausable => mint
            .get_extension::<PausableConfig>()
            .ok()
            .map(|ext| ParsedExtension::Mint(MintExtension::PausableConfig(*ext))),
        _ => None,
    }
}

/// Parse a mint extension string name to ExtensionType
pub fn parse_mint_extension_string(s: &str) -> Option<ExtensionType> {
    MintExtension::from_string(s)
}

/// Parse an account extension string name to ExtensionType  
pub fn parse_account_extension_string(s: &str) -> Option<ExtensionType> {
    AccountExtension::from_string(s)
}

/// Get all valid mint extension string names
pub fn get_all_mint_extension_names() -> &'static [&'static str] {
    MintExtension::all_string_names()
}

/// Get all valid account extension string names
pub fn get_all_account_extension_names() -> &'static [&'static str] {
    AccountExtension::all_string_names()
}
