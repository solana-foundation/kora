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

#[cfg(test)]
mod tests {
    use super::*;
    use spl_token_2022::extension::ExtensionType;

    #[test]
    fn test_mint_extension_from_string() {
        // Test valid mint extension strings
        assert_eq!(
            MintExtension::from_string("confidential_transfer_mint"),
            Some(ExtensionType::ConfidentialTransferMint)
        );
        assert_eq!(
            MintExtension::from_string("transfer_fee_config"),
            Some(ExtensionType::TransferFeeConfig)
        );
        assert_eq!(
            MintExtension::from_string("mint_close_authority"),
            Some(ExtensionType::MintCloseAuthority)
        );
        assert_eq!(
            MintExtension::from_string("interest_bearing_config"),
            Some(ExtensionType::InterestBearingConfig)
        );
        assert_eq!(
            MintExtension::from_string("non_transferable"),
            Some(ExtensionType::NonTransferable)
        );
        assert_eq!(
            MintExtension::from_string("permanent_delegate"),
            Some(ExtensionType::PermanentDelegate)
        );
        assert_eq!(MintExtension::from_string("transfer_hook"), Some(ExtensionType::TransferHook));
        assert_eq!(MintExtension::from_string("pausable"), Some(ExtensionType::Pausable));
        assert_eq!(
            MintExtension::from_string("confidential_mint_burn"),
            Some(ExtensionType::ConfidentialMintBurn)
        );

        // Test invalid strings
        assert_eq!(MintExtension::from_string("invalid_extension"), None);
        assert_eq!(MintExtension::from_string("memo_transfer"), None); // This is an account extension
        assert_eq!(MintExtension::from_string(""), None);
        assert_eq!(MintExtension::from_string("TRANSFER_FEE_CONFIG"), None); // Case sensitive
    }

    #[test]
    fn test_mint_extension_to_string_name() {
        // Test valid mint extension types
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::ConfidentialTransferMint),
            Some("confidential_transfer_mint")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::TransferFeeConfig),
            Some("transfer_fee_config")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::MintCloseAuthority),
            Some("mint_close_authority")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::InterestBearingConfig),
            Some("interest_bearing_config")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::NonTransferable),
            Some("non_transferable")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::PermanentDelegate),
            Some("permanent_delegate")
        );
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::TransferHook),
            Some("transfer_hook")
        );
        assert_eq!(MintExtension::to_string_name(ExtensionType::Pausable), Some("pausable"));
        assert_eq!(
            MintExtension::to_string_name(ExtensionType::ConfidentialMintBurn),
            Some("confidential_mint_burn")
        );

        // Test invalid extension types (account extensions)
        assert_eq!(MintExtension::to_string_name(ExtensionType::MemoTransfer), None);
        assert_eq!(MintExtension::to_string_name(ExtensionType::CpiGuard), None);
        assert_eq!(MintExtension::to_string_name(ExtensionType::ImmutableOwner), None);
    }

    #[test]
    fn test_mint_extension_all_string_names() {
        let names = MintExtension::all_string_names();

        // Check that all expected names are present
        let expected_names = [
            "confidential_transfer_mint",
            "confidential_mint_burn",
            "transfer_fee_config",
            "mint_close_authority",
            "interest_bearing_config",
            "non_transferable",
            "permanent_delegate",
            "transfer_hook",
            "pausable",
        ];

        assert_eq!(names.len(), expected_names.len());

        // Verify each expected name is present
        for expected_name in &expected_names {
            assert!(names.contains(expected_name), "Missing expected name: {expected_name}");
        }

        // Verify no account extension names are included
        assert!(!names.contains(&"memo_transfer"));
        assert!(!names.contains(&"cpi_guard"));
        assert!(!names.contains(&"immutable_owner"));
    }

    #[test]
    fn test_mint_extension_constants() {
        let extensions = MintExtension::EXTENSIONS;

        // Check that all expected extension types are present
        let expected_extensions = [
            ExtensionType::ConfidentialTransferMint,
            ExtensionType::ConfidentialMintBurn,
            ExtensionType::TransferFeeConfig,
            ExtensionType::MintCloseAuthority,
            ExtensionType::InterestBearingConfig,
            ExtensionType::NonTransferable,
            ExtensionType::PermanentDelegate,
            ExtensionType::TransferHook,
            ExtensionType::Pausable,
        ];

        assert_eq!(extensions.len(), expected_extensions.len());

        for expected_ext in &expected_extensions {
            assert!(
                extensions.contains(expected_ext),
                "Missing expected extension: {expected_ext:?}"
            );
        }

        // Verify no account extensions are included
        assert!(!extensions.contains(&ExtensionType::MemoTransfer));
        assert!(!extensions.contains(&ExtensionType::CpiGuard));
        assert!(!extensions.contains(&ExtensionType::ImmutableOwner));
    }

    #[test]
    fn test_account_extension_from_string() {
        // Test valid account extension strings
        assert_eq!(
            AccountExtension::from_string("confidential_transfer_account"),
            Some(ExtensionType::ConfidentialTransferAccount)
        );
        assert_eq!(
            AccountExtension::from_string("non_transferable_account"),
            Some(ExtensionType::NonTransferableAccount)
        );
        assert_eq!(
            AccountExtension::from_string("transfer_hook_account"),
            Some(ExtensionType::TransferHookAccount)
        );
        assert_eq!(
            AccountExtension::from_string("pausable_account"),
            Some(ExtensionType::PausableAccount)
        );
        assert_eq!(
            AccountExtension::from_string("memo_transfer"),
            Some(ExtensionType::MemoTransfer)
        );
        assert_eq!(AccountExtension::from_string("cpi_guard"), Some(ExtensionType::CpiGuard));
        assert_eq!(
            AccountExtension::from_string("immutable_owner"),
            Some(ExtensionType::ImmutableOwner)
        );
        assert_eq!(
            AccountExtension::from_string("default_account_state"),
            Some(ExtensionType::DefaultAccountState)
        );

        // Test invalid strings
        assert_eq!(AccountExtension::from_string("invalid_extension"), None);
        assert_eq!(AccountExtension::from_string("transfer_fee_config"), None); // This is a mint extension
        assert_eq!(AccountExtension::from_string(""), None);
        assert_eq!(AccountExtension::from_string("MEMO_TRANSFER"), None); // Case sensitive
    }

    #[test]
    fn test_account_extension_to_string_name() {
        // Test valid account extension types
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::ConfidentialTransferAccount),
            Some("confidential_transfer_account")
        );
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::NonTransferableAccount),
            Some("non_transferable_account")
        );
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::TransferHookAccount),
            Some("transfer_hook_account")
        );
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::PausableAccount),
            Some("pausable_account")
        );
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::MemoTransfer),
            Some("memo_transfer")
        );
        assert_eq!(AccountExtension::to_string_name(ExtensionType::CpiGuard), Some("cpi_guard"));
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::ImmutableOwner),
            Some("immutable_owner")
        );
        assert_eq!(
            AccountExtension::to_string_name(ExtensionType::DefaultAccountState),
            Some("default_account_state")
        );

        // Test invalid extension types (mint extensions)
        assert_eq!(AccountExtension::to_string_name(ExtensionType::TransferFeeConfig), None);
        assert_eq!(AccountExtension::to_string_name(ExtensionType::MintCloseAuthority), None);
        assert_eq!(AccountExtension::to_string_name(ExtensionType::NonTransferable), None);
    }

    #[test]
    fn test_account_extension_all_string_names() {
        let names = AccountExtension::all_string_names();

        // Check that all expected names are present
        let expected_names = [
            "confidential_transfer_account",
            "non_transferable_account",
            "transfer_hook_account",
            "pausable_account",
            "memo_transfer",
            "cpi_guard",
            "immutable_owner",
            "default_account_state",
        ];

        assert_eq!(names.len(), expected_names.len());

        // Verify each expected name is present
        for expected_name in &expected_names {
            assert!(names.contains(expected_name), "Missing expected name: {expected_name}");
        }

        // Verify no mint extension names are included
        assert!(!names.contains(&"transfer_fee_config"));
        assert!(!names.contains(&"mint_close_authority"));
        assert!(!names.contains(&"interest_bearing_config"));
    }

    #[test]
    fn test_account_extension_constants() {
        let extensions = AccountExtension::EXTENSIONS;

        // Check that all expected extension types are present
        let expected_extensions = [
            ExtensionType::ConfidentialTransferAccount,
            ExtensionType::NonTransferableAccount,
            ExtensionType::TransferHookAccount,
            ExtensionType::PausableAccount,
            ExtensionType::MemoTransfer,
            ExtensionType::CpiGuard,
            ExtensionType::ImmutableOwner,
            ExtensionType::DefaultAccountState,
        ];

        assert_eq!(extensions.len(), expected_extensions.len());

        for expected_ext in &expected_extensions {
            assert!(
                extensions.contains(expected_ext),
                "Missing expected extension: {expected_ext:?}"
            );
        }

        // Verify no mint extensions are included
        assert!(!extensions.contains(&ExtensionType::TransferFeeConfig));
        assert!(!extensions.contains(&ExtensionType::MintCloseAuthority));
        assert!(!extensions.contains(&ExtensionType::InterestBearingConfig));
    }

    #[test]
    fn test_parse_mint_extension_string() {
        // Test valid mint extension strings
        assert_eq!(
            parse_mint_extension_string("transfer_fee_config"),
            Some(ExtensionType::TransferFeeConfig)
        );
        assert_eq!(
            parse_mint_extension_string("mint_close_authority"),
            Some(ExtensionType::MintCloseAuthority)
        );
        assert_eq!(
            parse_mint_extension_string("non_transferable"),
            Some(ExtensionType::NonTransferable)
        );

        // Test invalid strings
        assert_eq!(parse_mint_extension_string("invalid_extension"), None);
        assert_eq!(parse_mint_extension_string("memo_transfer"), None); // Account extension
        assert_eq!(parse_mint_extension_string(""), None);
    }

    #[test]
    fn test_parse_account_extension_string() {
        // Test valid account extension strings
        assert_eq!(
            parse_account_extension_string("memo_transfer"),
            Some(ExtensionType::MemoTransfer)
        );
        assert_eq!(parse_account_extension_string("cpi_guard"), Some(ExtensionType::CpiGuard));
        assert_eq!(
            parse_account_extension_string("immutable_owner"),
            Some(ExtensionType::ImmutableOwner)
        );

        // Test invalid strings
        assert_eq!(parse_account_extension_string("invalid_extension"), None);
        assert_eq!(parse_account_extension_string("transfer_fee_config"), None); // Mint extension
        assert_eq!(parse_account_extension_string(""), None);
    }

    #[test]
    fn test_get_all_mint_extension_names() {
        let names = get_all_mint_extension_names();
        let direct_names = MintExtension::all_string_names();

        // Should be identical to the direct call
        assert_eq!(names, direct_names);

        // Should contain all expected mint extension names
        assert!(names.contains(&"transfer_fee_config"));
        assert!(names.contains(&"mint_close_authority"));
        assert!(names.contains(&"interest_bearing_config"));

        // Should not contain account extension names
        assert!(!names.contains(&"memo_transfer"));
        assert!(!names.contains(&"cpi_guard"));
    }

    #[test]
    fn test_get_all_account_extension_names() {
        let names = get_all_account_extension_names();
        let direct_names = AccountExtension::all_string_names();

        // Should be identical to the direct call
        assert_eq!(names, direct_names);

        // Should contain all expected account extension names
        assert!(names.contains(&"memo_transfer"));
        assert!(names.contains(&"cpi_guard"));
        assert!(names.contains(&"immutable_owner"));

        // Should not contain mint extension names
        assert!(!names.contains(&"transfer_fee_config"));
        assert!(!names.contains(&"mint_close_authority"));
    }

    #[test]
    fn test_extension_parsing_logic_coverage() {
        // Mint extensions that should be supported
        let supported_mint_extensions = [
            ExtensionType::ConfidentialTransferMint,
            ExtensionType::TransferFeeConfig,
            ExtensionType::MintCloseAuthority,
            ExtensionType::InterestBearingConfig,
            ExtensionType::NonTransferable,
            ExtensionType::PermanentDelegate,
            ExtensionType::TransferHook,
            ExtensionType::Pausable,
            ExtensionType::ConfidentialMintBurn,
        ];

        // Account extensions that should be supported
        let supported_account_extensions = [
            ExtensionType::ConfidentialTransferAccount,
            ExtensionType::NonTransferableAccount,
            ExtensionType::TransferHookAccount,
            ExtensionType::PausableAccount,
            ExtensionType::MemoTransfer,
            ExtensionType::CpiGuard,
            ExtensionType::ImmutableOwner,
            ExtensionType::DefaultAccountState,
        ];

        // Verify that the constants match our expected extensions
        assert_eq!(MintExtension::EXTENSIONS.len(), supported_mint_extensions.len());
        assert_eq!(AccountExtension::EXTENSIONS.len(), supported_account_extensions.len());

        // Verify all supported mint extensions are in our constants
        for ext in supported_mint_extensions {
            assert!(MintExtension::EXTENSIONS.contains(&ext));
        }

        // Verify all supported account extensions are in our constants
        for ext in supported_account_extensions {
            assert!(AccountExtension::EXTENSIONS.contains(&ext));
        }
    }
}
