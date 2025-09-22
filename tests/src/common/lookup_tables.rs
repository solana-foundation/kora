use anyhow::Result;
use solana_address_lookup_table_interface::instruction::{
    create_lookup_table, extend_lookup_table,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use std::{str::FromStr, sync::Arc};

use crate::common::{constants::*, SenderTestHelper, USDCMintTestHelper};

/// Comprehensive helper for all lookup table operations in tests
pub struct LookupTableHelper;

impl LookupTableHelper {
    // ============================================================================
    // Fixtures Management
    // ============================================================================

    /// Create all standard lookup tables and save addresses to fixtures
    pub async fn setup_and_save_lookup_tables(
        rpc_client: Arc<RpcClient>,
    ) -> Result<(Pubkey, Pubkey, Pubkey)> {
        let sender = SenderTestHelper::get_test_sender_keypair();

        // Create all standard lookup tables
        let allowed_lookup_table =
            Self::create_allowed_lookup_table(rpc_client.clone(), &sender).await?;
        let disallowed_lookup_table =
            Self::create_disallowed_lookup_table(rpc_client.clone(), &sender).await?;
        let transaction_lookup_table =
            Self::create_transaction_lookup_table(rpc_client.clone(), &sender).await?;

        Ok((allowed_lookup_table, disallowed_lookup_table, transaction_lookup_table))
    }

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

    // ============================================================================
    // Core Lookup Table Creation
    // ============================================================================

    /// Create a lookup table with specified addresses
    pub async fn create_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
        addresses: Vec<Pubkey>,
    ) -> Result<Pubkey> {
        let recent_slot = rpc_client.get_slot().await?;

        // Create the lookup table
        let (create_instruction, lookup_table_key) =
            create_lookup_table(authority.pubkey(), authority.pubkey(), recent_slot - 1);

        let recent_blockhash = rpc_client.get_latest_blockhash().await?;

        let create_transaction = Transaction::new_signed_with_payer(
            &[create_instruction],
            Some(&authority.pubkey()),
            &[authority],
            recent_blockhash,
        );

        rpc_client.send_and_confirm_transaction(&create_transaction).await?;

        // Add addresses to the lookup table
        if !addresses.is_empty() {
            let extend_instruction = extend_lookup_table(
                lookup_table_key,
                authority.pubkey(),
                Some(authority.pubkey()),
                addresses.clone(),
            );

            let recent_blockhash = rpc_client.get_latest_blockhash().await?;

            let extend_transaction = Transaction::new_signed_with_payer(
                &[extend_instruction],
                Some(&authority.pubkey()),
                &[authority],
                recent_blockhash,
            );

            rpc_client.send_and_confirm_transaction(&extend_transaction).await?;
        }

        // Wait for the lookup table to be activated
        // Lookup tables need to be activated for at least one slot before they can be used
        let creation_slot = rpc_client.get_slot().await?;
        let mut current_slot = creation_slot;

        // Wait until we're at least 2 slots past creation to ensure activation
        while current_slot <= creation_slot + 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            current_slot = rpc_client.get_slot().await?;
        }

        Ok(lookup_table_key)
    }

    // ============================================================================
    // Allowed / Disallowed addresses in lookup tables
    // ============================================================================

    pub async fn create_allowed_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
    ) -> Result<Pubkey> {
        let allowed_lookup_table =
            Self::create_lookup_table(rpc_client, authority, vec![solana_sdk::system_program::ID])
                .await?;

        Ok(allowed_lookup_table)
    }

    pub async fn create_disallowed_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
    ) -> Result<Pubkey> {
        let disallowed_address = Self::get_test_disallowed_address()?;
        let blocked_lookup_table: Pubkey =
            Self::create_lookup_table(rpc_client, authority, vec![disallowed_address]).await?;

        Ok(blocked_lookup_table)
    }

    // ============================================================================
    // Transaction-Specific Lookup Tables (for SPL transfers with mint)
    // ============================================================================
    pub async fn create_transaction_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
    ) -> Result<Pubkey> {
        let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

        let addresses = vec![usdc_mint, spl_token::ID];

        Self::create_lookup_table(rpc_client, authority, addresses).await
    }
}
