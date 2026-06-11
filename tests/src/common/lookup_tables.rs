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

        // Right after validator boot the earliest slots may be missing from the
        // SlotHashes sysvar, which CreateLookupTable validates derivation slots against
        let mut current_slot = rpc_client.get_slot().await?;
        while current_slot < 10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            current_slot = rpc_client.get_slot().await?;
        }

        // Distinct derivation slots so concurrently created tables get distinct addresses
        let base_slot = current_slot.saturating_sub(3);

        tokio::try_join!(
            Self::create_allowed_lookup_table(rpc_client.clone(), &sender, base_slot + 2),
            Self::create_disallowed_lookup_table(rpc_client.clone(), &sender, base_slot + 1),
            Self::create_transaction_lookup_table(rpc_client.clone(), &sender, base_slot),
        )
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
        derivation_slot: u64,
    ) -> Result<Pubkey> {
        let (create_instruction, lookup_table_key) =
            create_lookup_table(authority.pubkey(), authority.pubkey(), derivation_slot);

        let mut instructions = vec![create_instruction];
        if !addresses.is_empty() {
            instructions.push(extend_lookup_table(
                lookup_table_key,
                authority.pubkey(),
                Some(authority.pubkey()),
                addresses,
            ));
        }

        let recent_blockhash = rpc_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&authority.pubkey()),
            &[authority],
            recent_blockhash,
        );

        rpc_client.send_and_confirm_transaction(&transaction).await?;

        // Lookup tables need to be activated for at least one slot before they can be used
        let creation_slot = rpc_client.get_slot().await?;
        let mut current_slot = creation_slot;

        while current_slot <= creation_slot + 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
        derivation_slot: u64,
    ) -> Result<Pubkey> {
        Self::create_lookup_table(
            rpc_client,
            authority,
            vec![solana_system_interface::program::ID],
            derivation_slot,
        )
        .await
    }

    pub async fn create_disallowed_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
        derivation_slot: u64,
    ) -> Result<Pubkey> {
        let disallowed_address = Self::get_test_disallowed_address()?;
        Self::create_lookup_table(rpc_client, authority, vec![disallowed_address], derivation_slot)
            .await
    }

    // ============================================================================
    // Transaction-Specific Lookup Tables (for SPL transfers with mint)
    // ============================================================================
    pub async fn create_transaction_lookup_table(
        rpc_client: Arc<RpcClient>,
        authority: &Keypair,
        derivation_slot: u64,
    ) -> Result<Pubkey> {
        let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

        let addresses = vec![usdc_mint, spl_token_interface::ID];

        Self::create_lookup_table(rpc_client, authority, addresses, derivation_slot).await
    }
}
