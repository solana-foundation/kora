//! Jito bundle support for Kora
//!
//! This module provides functionality for creating, validating, and sending
//! transaction bundles through the Jito block engine for MEV protection
//! and atomic transaction execution.

pub mod client;
pub mod tip;
pub mod types;

pub use client::JitoClient;
pub use tip::{
    add_tip_to_transaction, create_tip_instruction, find_tip_in_bundle, get_random_tip_account,
};
pub use types::{Bundle, BundleValidationResult, SendBundleResponse};

use crate::constant::JITO_TIP_ACCOUNTS;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Returns the list of valid Jito tip account pubkeys
pub fn get_tip_accounts() -> Vec<Pubkey> {
    JITO_TIP_ACCOUNTS.iter().filter_map(|s| Pubkey::from_str(s).ok()).collect()
}

/// Checks if a given pubkey is a valid Jito tip account
pub fn is_tip_account(pubkey: &Pubkey) -> bool {
    JITO_TIP_ACCOUNTS
        .iter()
        .any(|s| Pubkey::from_str(s).map(|tip_account| tip_account == *pubkey).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tip_accounts() {
        let accounts = get_tip_accounts();
        assert_eq!(accounts.len(), 8);
    }

    #[test]
    fn test_is_tip_account() {
        let valid_tip = Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap();
        assert!(is_tip_account(&valid_tip));

        let invalid_tip = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        assert!(!is_tip_account(&invalid_tip));
    }
}
