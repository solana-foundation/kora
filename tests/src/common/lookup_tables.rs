use anyhow::Result;
use solana_address_lookup_table_interface::instruction::{
    create_lookup_table, extend_lookup_table,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use std::{
    str::FromStr,
    sync::{Arc, OnceLock},
};

use crate::common::{constants::*, SenderTestHelper, USDCMintTestHelper};

// Static cache for lookup table addresses - loaded once, reused everywhere
pub static CACHED_ADDRESSES: OnceLock<LookupTablesAddresses> = OnceLock::new();

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct LookupTablesAddresses {
    pub allowed_lookup_table_address: String,
    pub disallowed_lookup_table_address: String,
    pub transaction_lookup_table_address: String,
}

/// Comprehensive helper for all lookup table operations in tests
pub struct LookupTableHelper;

impl LookupTablesAddresses {
    /// Save lookup table addresses to JSON file
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(LOOKUP_TABLES_FILE_PATH, json)?;
        Ok(())
    }

    /// Load lookup table addresses from JSON file
    pub fn load() -> Result<Self> {
        let json = std::fs::read_to_string(LOOKUP_TABLES_FILE_PATH)?;
        let addresses: LookupTablesAddresses = serde_json::from_str(&json)?;
        Ok(addresses)
    }

    pub fn allowed_pubkey(&self) -> Result<Pubkey> {
        Pubkey::from_str(&self.allowed_lookup_table_address).map_err(Into::into)
    }

    pub fn disallowed_pubkey(&self) -> Result<Pubkey> {
        Pubkey::from_str(&self.disallowed_lookup_table_address).map_err(Into::into)
    }

    pub fn transaction_pubkey(&self) -> Result<Pubkey> {
        Pubkey::from_str(&self.transaction_lookup_table_address).map_err(Into::into)
    }

    /// Create from Pubkeys for saving
    pub fn from_pubkeys(allowed: Pubkey, disallowed: Pubkey, transaction: Pubkey) -> Self {
        Self {
            allowed_lookup_table_address: allowed.to_string(),
            disallowed_lookup_table_address: disallowed.to_string(),
            transaction_lookup_table_address: transaction.to_string(),
        }
    }
}

impl LookupTableHelper {
    // ============================================================================
    // Fixtures Management
    // ============================================================================

    /// Create all standard lookup tables and save addresses to fixtures
    pub async fn setup_and_save_lookup_tables(rpc_client: Arc<RpcClient>) -> Result<()> {
        let sender = SenderTestHelper::get_test_sender_keypair();

        // Create all standard lookup tables
        let allowed_lookup_table =
            Self::create_allowed_lookup_table(rpc_client.clone(), &sender).await?;
        let disallowed_lookup_table =
            Self::create_disallowed_lookup_table(rpc_client.clone(), &sender).await?;
        let transaction_lookup_table =
            Self::create_transaction_lookup_table(rpc_client.clone(), &sender).await?;

        // Save addresses to JSON file
        let addresses = LookupTablesAddresses::from_pubkeys(
            allowed_lookup_table,
            disallowed_lookup_table,
            transaction_lookup_table,
        );
        addresses.save()?;

        Ok(())
    }

    fn load_lookup_table_addresses() -> Result<&'static LookupTablesAddresses> {
        if let Some(cached) = CACHED_ADDRESSES.get() {
            return Ok(cached);
        }

        let addresses = LookupTablesAddresses::load()?;

        let _ = CACHED_ADDRESSES.set(addresses);

        Ok(CACHED_ADDRESSES.get().unwrap())
    }

    /// Get allowed lookup table address from fixtures
    pub fn get_allowed_lookup_table_address() -> Result<Pubkey> {
        let addresses = Self::load_lookup_table_addresses()?;
        addresses.allowed_pubkey()
    }

    /// Get disallowed lookup table address from fixtures
    pub fn get_disallowed_lookup_table_address() -> Result<Pubkey> {
        let addresses = Self::load_lookup_table_addresses()?;
        addresses.disallowed_pubkey()
    }

    /// Get transaction lookup table address from fixtures
    pub fn get_transaction_lookup_table_address() -> Result<Pubkey> {
        let addresses = Self::load_lookup_table_addresses()?;
        addresses.transaction_pubkey()
    }

    /// Get test disallowed address (for creating custom lookup tables)
    pub fn get_test_disallowed_address() -> Pubkey {
        get_test_disallowed_pubkey()
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

        // Wait for the lookup table to be fully initialized
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

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
        let disallowed_address = get_test_disallowed_pubkey();
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
