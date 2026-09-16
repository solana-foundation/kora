use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::common::constants::*;

/// Resolves the lookup tables the harness creates, from the env vars it exports
pub struct LookupTableHelper;

impl LookupTableHelper {
    pub fn get_test_disallowed_address() -> Result<Pubkey> {
        Pubkey::from_str(TEST_DISALLOWED_ADDRESS).map_err(Into::into)
    }

    pub fn get_allowed_lookup_table_address() -> Result<Pubkey> {
        dotenv::dotenv().ok();
        let allowed_lookup_table_address = std::env::var(TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV)
            .expect("TEST_ALLOWED_LOOKUP_TABLE_ADDRESS environment variable is not set");
        Pubkey::from_str(&allowed_lookup_table_address).map_err(Into::into)
    }

    pub fn get_disallowed_lookup_table_address() -> Result<Pubkey> {
        dotenv::dotenv().ok();
        let disallowed_lookup_table_address =
            std::env::var(TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV)
                .expect("TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS environment variable is not set");
        Pubkey::from_str(&disallowed_lookup_table_address).map_err(Into::into)
    }

    pub fn get_transaction_lookup_table_address() -> Result<Pubkey> {
        dotenv::dotenv().ok();
        let transaction_lookup_table_address =
            std::env::var(TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV)
                .expect("TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS environment variable is not set");
        Pubkey::from_str(&transaction_lookup_table_address).map_err(Into::into)
    }
}
