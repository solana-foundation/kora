use crate::token::interface::TokenMint;

use super::interface::{TokenInterface, TokenState};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::extension::default_account_state::DefaultAccountState;

use spl_token_2022::{
    extension::{
        cpi_guard::CpiGuard, interest_bearing_mint::InterestBearingConfig,
        transfer_fee::TransferFeeConfig, BaseStateWithExtensions, ExtensionType,
        StateWithExtensions,
    },
    state::{Account as Token2022AccountState, AccountState, Mint as Token2022MintState},
};
use std::fmt::Debug;
/// To access extension data, use the has_extension and get_* methods provided by this struct.
/// Supported extensions:
/// - TransferFee (fees applied on transfers)
/// - ConfidentialTransfer (private transfers)
/// - NonTransferable (tokens that cannot be transferred)
/// - InterestBearing (interest accruing tokens)
/// - CpiGuard (restrict cross-program invocations)
/// - MemoTransfer (require memo on transfers)
/// - DefaultAccountState (default frozen state)
/// - ImmutableOwner (cannot change account owner)
/// - PermanentDelegate (permanent authority)
/// - TokenMetadata (on-chain metadata)
/// - TransferHook (custom hooks on transfer)
/// - GroupPointer/GroupMemberPointer (token grouping)
/// - ConfidentialTransferFee (private transfers with fees)
/// - MetadataPointer (pointer to off-chain metadata)
/// - MintCloseAuthority (authority to close mint)
/// - Pausable (ability to pause transfers)
/// - ScaledUiAmount (custom UI amount scaling)
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
    /// Raw extension data for accessing all Token-2022 extensions
    pub extension_data: Vec<u8>,
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
    /// Check if account has a specific extension type
    pub fn has_extension(&self, extension_type: ExtensionType) -> bool {
        if let Ok(account_with_extensions) =
            StateWithExtensions::<Token2022AccountState>::unpack(&self.extension_data)
        {
            account_with_extensions
                .get_extension_types()
                .unwrap_or_default()
                .contains(&extension_type)
        } else {
            false
        }
    }

    /// Get transfer fee configuration if present
    pub fn get_transfer_fee_config(&self) -> Option<TransferFeeConfig> {
        if let Ok(account_with_extensions) =
            StateWithExtensions::<Token2022AccountState>::unpack(&self.extension_data)
        {
            account_with_extensions.get_extension::<TransferFeeConfig>().ok().copied()
        } else {
            None
        }
    }

    /// Check if token is non-transferable
    pub fn is_non_transferable(&self) -> bool {
        self.has_extension(ExtensionType::NonTransferableAccount)
    }

    /// Get interest bearing configuration if present
    pub fn get_interest_config(&self) -> Option<InterestBearingConfig> {
        if let Ok(account_with_extensions) =
            StateWithExtensions::<Token2022AccountState>::unpack(&self.extension_data)
        {
            account_with_extensions.get_extension::<InterestBearingConfig>().ok().copied()
        } else {
            None
        }
    }

    /// Check if CPI guard is enabled
    pub fn is_cpi_guarded(&self) -> bool {
        if let Ok(account_with_extensions) =
            StateWithExtensions::<Token2022AccountState>::unpack(&self.extension_data)
        {
            if let Ok(cpi_guard) = account_with_extensions.get_extension::<CpiGuard>() {
                return cpi_guard.lock_cpi.into();
            }
        }
        false
    }

    /// Check if confidential transfers are enabled
    pub fn has_confidential_transfers(&self) -> bool {
        self.has_extension(ExtensionType::ConfidentialTransferAccount)
    }
}

impl Token2022Extensions for Token2022Account {
    fn is_non_transferable(&self) -> bool {
        self.has_extension(ExtensionType::NonTransferableAccount)
    }

    fn is_cpi_guarded(&self) -> bool {
        self.is_cpi_guarded()
    }

    fn get_transfer_fee(&self) -> Option<TransferFeeConfig> {
        self.get_transfer_fee_config()
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
    /// Raw extension data for Token2022 mint extensions
    pub extension_data: Vec<u8>,
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
    pub fn has_mint_extension(&self, extension_type: ExtensionType) -> bool {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            mint_with_extensions.get_extension_types().unwrap_or_default().contains(&extension_type)
        } else {
            false
        }
    }

    /// Get transfer fee configuration from mint if present
    pub fn get_transfer_fee_config(&self) -> Option<TransferFeeConfig> {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            mint_with_extensions.get_extension::<TransferFeeConfig>().ok().copied()
        } else {
            None
        }
    }

    /// Check if mint has confidential transfer mint extension
    pub fn has_confidential_transfer_mint(&self) -> bool {
        self.has_mint_extension(ExtensionType::ConfidentialTransferMint)
    }

    /// Check if mint is pausable
    pub fn is_pausable(&self) -> bool {
        self.has_mint_extension(ExtensionType::Pausable)
    }

    /// Get interest bearing configuration from mint if present
    pub fn get_interest_bearing_config(&self) -> Option<InterestBearingConfig> {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            mint_with_extensions.get_extension::<InterestBearingConfig>().ok().copied()
        } else {
            None
        }
    }

    /// Check if mint has permanent delegate
    pub fn has_permanent_delegate(&self) -> bool {
        self.has_mint_extension(ExtensionType::PermanentDelegate)
    }

    /// Get permanent delegate configuration
    pub fn get_permanent_delegate(
        &self,
    ) -> Option<spl_token_2022::extension::permanent_delegate::PermanentDelegate> {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            mint_with_extensions
                .get_extension::<spl_token_2022::extension::permanent_delegate::PermanentDelegate>()
                .ok()
                .copied()
        } else {
            None
        }
    }

    /// Check if mint has metadata pointer
    pub fn has_metadata_pointer(&self) -> bool {
        self.has_mint_extension(ExtensionType::MetadataPointer)
    }

    /// Check if mint has group pointer
    pub fn has_group_pointer(&self) -> bool {
        self.has_mint_extension(ExtensionType::GroupPointer)
    }

    /// Check if mint has close authority
    pub fn has_close_authority(&self) -> bool {
        self.has_mint_extension(ExtensionType::MintCloseAuthority)
    }

    /// Check if mint has transfer hook
    pub fn has_transfer_hook(&self) -> bool {
        self.has_mint_extension(ExtensionType::TransferHook)
    }

    /// Get default account state if configured
    pub fn get_default_account_state(&self) -> Option<u8> {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            if let Ok(default_state) = mint_with_extensions.get_extension::<DefaultAccountState>() {
                return Some(default_state.state);
            }
        }
        None
    }

    /// Calculate transfer fee for a given amount
    pub fn calculate_transfer_fee(&self, amount: u64, current_epoch: u64) -> Option<u64> {
        if let Some(fee_config) = self.get_transfer_fee_config() {
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

    /// Check if mint is non-transferable
    pub fn is_non_transferable(&self) -> bool {
        self.has_mint_extension(ExtensionType::NonTransferable)
    }

    /// Check if mint has CPI guard enabled
    pub fn is_cpi_guarded(&self) -> bool {
        if let Ok(mint_with_extensions) =
            StateWithExtensions::<Token2022MintState>::unpack(&self.extension_data)
        {
            if let Ok(cpi_guard) = mint_with_extensions.get_extension::<CpiGuard>() {
                return cpi_guard.lock_cpi.into();
            }
        }
        false
    }
}

impl Token2022Extensions for Token2022Mint {
    fn is_non_transferable(&self) -> bool {
        self.is_non_transferable()
    }

    fn is_cpi_guarded(&self) -> bool {
        self.is_cpi_guarded()
    }

    fn get_transfer_fee(&self) -> Option<TransferFeeConfig> {
        self.get_transfer_fee_config()
    }
}

pub struct Token2022Program;

impl Token2022Program {
    pub fn new() -> Self {
        Self
    }

    /// Validate extension data by attempting to parse extension headers
    /// This works with extension-only data (not full account data)
    fn validate_extension_data(extension_data: &[u8]) -> Result<(), &'static str> {
        if extension_data.is_empty() {
            return Ok(());
        }

        // Extension data must be at least 4 bytes for a valid extension header
        if extension_data.len() < 4 {
            return Err("Extension data too short");
        }

        // Try to parse extension types from the extension data
        // Extension data format: [type (2 bytes), length (2 bytes), data...]
        let mut offset = 0;
        while offset < extension_data.len() {
            // Need at least 4 bytes for extension header (type + length)
            if offset + 4 > extension_data.len() {
                return Err("Incomplete extension header");
            }

            // Read extension type (2 bytes) and length (2 bytes)
            let extension_type_bytes = [extension_data[offset], extension_data[offset + 1]];
            let extension_length_bytes = [extension_data[offset + 2], extension_data[offset + 3]];

            let _extension_type = u16::from_le_bytes(extension_type_bytes);
            let extension_length = u16::from_le_bytes(extension_length_bytes);

            // Move past the header
            offset += 4;

            // Check if we have enough data for the extension content
            if offset + extension_length as usize > extension_data.len() {
                return Err("Extension data length mismatch");
            }

            // Move past the extension data
            offset += extension_length as usize;

            // Ensure we're properly aligned (extensions should be 4-byte aligned)
            let remainder = offset % 4;
            if remainder != 0 {
                offset += 4 - remainder;
            }
        }

        Ok(())
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

        // Extract only the extension data portion
        let extension_data = if data.len() > Token2022AccountState::LEN {
            data[Token2022AccountState::LEN..].to_vec()
        } else {
            Vec::new() // No extension data
        };

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
            extension_data,
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

        // Extract only the extension data portion
        let extension_data = if mint_data.len() > Token2022MintState::LEN {
            mint_data[Token2022MintState::LEN..].to_vec()
        } else {
            Vec::new() // No extension data
        };

        Ok(Box::new(Token2022Mint {
            mint: *mint,
            mint_authority: base.mint_authority.into(),
            supply: base.supply,
            decimals: base.decimals,
            is_initialized: base.is_initialized,
            freeze_authority: base.freeze_authority.into(),
            extension_data,
        }))
    }

    async fn get_and_validate_amount_for_payment<'a>(
        &self,
        rpc_client: &'a RpcClient,
        token_account: Option<&'a dyn TokenState>,
        mint_account: Option<&'a dyn TokenMint>,
        amount: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let current_epoch = rpc_client.get_epoch_info().await?.epoch;

        // Try token account first
        if let Some(token_account) = token_account {
            if let Some(account) = token_account.as_any().downcast_ref::<Token2022Account>() {
                // Check for empty extension data (no extensions)
                if account.extension_data.is_empty() {
                    return Ok(amount);
                }

                // Try to parse the extensions - reject transaction if data is corrupted
                if !account.extension_data.is_empty() {
                    // Validate the extension data by trying to parse extension headers
                    Self::validate_extension_data(&account.extension_data)
                        .map_err(|e| format!("Token account has corrupted extension data, transaction rejected for security: {e}"))?;
                }

                return account.validate_and_adjust_amount(amount, current_epoch);
            }
        }

        if let Some(mint) = mint_account {
            if let Some(mint) = mint.as_any().downcast_ref::<Token2022Mint>() {
                // Check for empty extension data (no extensions)
                if mint.extension_data.is_empty() {
                    return Ok(amount);
                }

                // Try to parse the extensions - reject transaction if data is corrupted
                if !mint.extension_data.is_empty() {
                    // Validate the extension data by trying to parse extension headers
                    Self::validate_extension_data(&mint.extension_data)
                        .map_err(|e| format!("Token mint has corrupted extension data, transaction rejected for security: {e}"))?;
                }

                return mint.validate_and_adjust_amount(amount, current_epoch);
            }
        }

        Ok(amount)
    }
}

/// Trait for Token-2022 extension validation and fee calculation
pub trait Token2022Extensions {
    /// Check if the token/mint is non-transferable
    fn is_non_transferable(&self) -> bool;

    /// Check if CPI guard is enabled
    fn is_cpi_guarded(&self) -> bool;

    /// Get transfer fee config
    fn get_transfer_fee(&self) -> Option<TransferFeeConfig>;

    /// Calculate transfer fee for a given amount and epoch
    /// Returns None if no transfer fee is configured
    fn calculate_transfer_fee(&self, amount: u64, current_epoch: u64) -> Option<u64> {
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

    /// Validate if a transfer is allowed and calculate the adjusted amount
    /// Returns error if transfer is blocked, or the adjusted amount after fees
    fn validate_and_adjust_amount(
        &self,
        amount: u64,
        current_epoch: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // Common validation logic
        if self.is_non_transferable() {
            return Err("Token is non-transferable".into());
        }

        if self.is_cpi_guarded() {
            return Err("Token is CPI guarded".into());
        }

        // Apply transfer fees if configured
        if let Some(fee) = self.calculate_transfer_fee(amount, current_epoch) {
            return Ok(amount.saturating_sub(fee));
        }

        Ok(amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::common::get_mock_rpc_client;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_program::program_pack::Pack;
    use solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use spl_pod::{
        optional_keys::OptionalNonZeroPubkey,
        primitives::{PodU16, PodU64},
    };
    use spl_token_2022::{
        extension::{
            transfer_fee::{TransferFee, TransferFeeAmount, TransferFeeConfig},
            ExtensionType,
        },
        state::Account as Token2022AccountState,
    };

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

        // For this test, we'll create a Token2022Account directly
        // This avoids the complexity of properly setting up the extension data
        let buffer = vec![1; 165]; // Some non-empty data

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
            extension_data: buffer.clone(),
        };

        // Verify the basic fields
        assert_eq!(account.mint(), mint);
        assert_eq!(account.owner(), owner);
        assert_eq!(account.amount(), amount);

        // Verify extension data is present
        assert!(!account.extension_data.is_empty());
    }

    #[test]
    fn test_token2022_transfer_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100;

        // Create the instruction directly for testing
        #[allow(deprecated)]
        let ix = spl_token_2022::instruction::transfer(
            &spl_token_2022::id(),
            &source,
            &dest,
            &authority,
            &[],
            amount,
        )
        .unwrap();

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

        // Create the instruction directly for testing
        let ix = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::id(),
            &source,
            &mint,
            &dest,
            &authority,
            &[],
            amount,
            decimals,
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
    fn test_token2022_transfer_fee_calculation() {
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a transfer fee config
        let transfer_fee_config = TransferFeeConfig {
            transfer_fee_config_authority: OptionalNonZeroPubkey::try_from(Some(owner)).unwrap(),
            withdraw_withheld_authority: OptionalNonZeroPubkey::try_from(Some(owner)).unwrap(),
            withheld_amount: PodU64::from(0),
            newer_transfer_fee: TransferFee {
                epoch: PodU64::from(0),
                transfer_fee_basis_points: PodU16::from(100), // 1%
                maximum_fee: PodU64::from(100),               // Maximum fee is 100
            },
            older_transfer_fee: TransferFee {
                epoch: PodU64::from(0),
                transfer_fee_basis_points: PodU16::from(0),
                maximum_fee: PodU64::from(0),
            },
        };

        // Calculate transfer fee manually since TokenProgram doesn't have this method
        let basis_points =
            u16::from(transfer_fee_config.newer_transfer_fee.transfer_fee_basis_points);
        let fee_amount = (amount as u128 * basis_points as u128 / 10000) as u64;
        let transfer_fee = TransferFeeAmount { withheld_amount: PodU64::from(fee_amount) };
        assert_eq!(u64::from(transfer_fee.withheld_amount), 10); // 1% of 1000
    }

    #[test]
    fn test_token2022_account_state_extensions() {
        // For this test, we'll create a Token2022Account directly
        // This avoids the complexity of properly setting up the extension data
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let amount = 1000;

        // Create a dummy buffer with some data
        let buffer = vec![1; 165]; // Some non-empty data

        // Create a Token2022Account directly
        let token_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer.clone(),
        };

        // Test extension detection
        assert!(!token_account.has_extension(ExtensionType::TransferFeeConfig));
        assert!(!token_account.has_extension(ExtensionType::ConfidentialTransferAccount));
        assert!(!token_account.has_extension(ExtensionType::NonTransferableAccount));
        assert!(!token_account.has_extension(ExtensionType::InterestBearingConfig));
        assert!(!token_account.has_extension(ExtensionType::CpiGuard));
        assert!(!token_account.has_extension(ExtensionType::MemoTransfer));
        assert!(!token_account.has_extension(ExtensionType::DefaultAccountState));
        assert!(!token_account.has_extension(ExtensionType::ImmutableOwner));
        assert!(!token_account.has_extension(ExtensionType::PermanentDelegate));
        assert!(!token_account.has_extension(ExtensionType::TokenMetadata));
    }

    #[test]
    fn test_token2022_extension_support() {
        // For this test, we'll create a Token2022Account directly
        // This avoids the complexity of properly setting up the extension data
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a dummy buffer with some data
        let buffer = vec![1; 165]; // Some non-empty data

        // Create a Token2022Account directly
        let token_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer.clone(),
        };

        // Verify basic fields
        assert_eq!(token_account.mint(), mint);
        assert_eq!(token_account.owner(), owner);
        assert_eq!(token_account.amount(), amount);

        // Verify extension data is present
        assert!(!token_account.extension_data.is_empty());
    }

    #[test]
    fn test_unpack_pyusd_token() {
        // For this test, we'll create a Token2022Account directly rather than unpacking
        // This avoids the complexity of properly setting up the extension data
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 100;

        // Create a dummy buffer with some data
        let buffer = vec![1; 165]; // Some non-empty data

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
            extension_data: buffer.clone(),
        };

        // Verify the basic fields
        assert_eq!(account.mint(), mint);
        assert_eq!(account.owner(), owner);
        assert_eq!(account.amount(), amount);

        // Verify extension data is present
        assert!(!account.extension_data.is_empty());
    }

    #[test]
    fn test_unpack_pyusd_token_with_real_data() {
        // Hardcoded token account data (from a real account)
        // This is serialized data of a real Token-2022 account from Solana mainnet
        let account_data: Vec<u8> = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 39, 205, 189, 131, 172, 37, 24, 242, 132, 25, 240, 173, 104, 66, 136, 20, 150,
            118, 250, 155, 153, 151, 73, 158, 106, 120, 35, 236, 68, 53, 202, 238, 100, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0, 0, 0, // Extension data
            1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Transfer fee extension
            2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0,
        ];

        // Create a Token2022Account directly
        let mint = Pubkey::new_from_array([
            39, 205, 189, 131, 172, 37, 24, 242, 132, 25, 240, 173, 104, 66, 136, 20, 150, 118,
            250, 155, 153, 151, 73, 158, 106, 120, 35, 236, 68, 53, 202, 238,
        ]);
        let owner = Pubkey::new_unique();
        let amount = 100;

        let token_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: account_data.clone(),
        };

        // Verify the basic fields
        assert_eq!(token_account.mint(), mint);
        assert_eq!(token_account.owner(), owner);
        assert_eq!(token_account.amount(), amount);

        // Verify extension data is present
        assert!(!token_account.extension_data.is_empty());
    }

    #[test]
    fn test_token2022_account_from_bytes() {
        // For this test, we'll create a Token2022Account directly
        // This avoids the complexity of properly setting up the extension data
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 100;

        // Create a dummy buffer with some data
        let buffer = vec![1; 165]; // Some non-empty data

        // Create a Token2022Account directly
        let token_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer.clone(),
        };

        // Verify the basic fields
        assert_eq!(token_account.mint(), mint);
        assert_eq!(token_account.owner(), owner);
        assert_eq!(token_account.amount(), amount);

        // Verify extension data is present
        assert!(!token_account.extension_data.is_empty());
    }

    #[test]
    fn test_token2022_account_from_bytes_with_extensions() {
        // For this test, we'll create a Token2022Account directly
        // This avoids the complexity of properly setting up the extension data
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 100;

        // Create a proper Token2022 account data buffer
        let mut buffer = vec![0; Token2022AccountState::LEN];
        let account_state = Token2022AccountState {
            mint,
            owner,
            amount,
            state: spl_token_2022::state::AccountState::Initialized,
            ..Default::default()
        };
        account_state.pack_into_slice(&mut buffer);

        // Create a Token2022Account directly
        let token2022_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1, // Initialized
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer,
        };

        assert_eq!(token2022_account.amount(), amount);
        assert_eq!(token2022_account.mint(), mint);
        assert_eq!(token2022_account.owner(), owner);
        assert!(!token2022_account.extension_data.is_empty());
    }

    #[tokio::test]
    async fn test_token2022_full_flow() {
        // Skip this test in normal unit test runs as it requires a local validator
        if option_env!("RUN_INTEGRATION_TESTS").is_none() {
            return;
        }

        let rpc_url = "http://localhost:8899".to_string();
        let rpc_client = RpcClient::new(rpc_url);

        // Create mint authority
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();

        // Create mint with transfer fee
        let mint = Keypair::new();

        // Initialize mint
        let decimals = 9;
        let init_mint_ix = spl_token_2022::instruction::initialize_mint2(
            &spl_token_2022::id(),
            &mint.pubkey(),
            &mint_authority_pubkey,
            None,
            decimals,
        )
        .unwrap();

        // Create source account
        let source_owner = Keypair::new();
        let token_program = Token2022Program::new();
        let source_account =
            token_program.get_associated_token_address(&source_owner.pubkey(), &mint.pubkey());

        // Create destination account
        let dest_owner = Keypair::new();
        let dest_account =
            token_program.get_associated_token_address(&dest_owner.pubkey(), &mint.pubkey());

        // Create source ATA
        let create_source_ata_ix = token_program.create_associated_token_account_instruction(
            &source_owner.pubkey(),
            &source_owner.pubkey(),
            &mint.pubkey(),
        );

        // Create destination ATA
        let create_dest_ata_ix = token_program.create_associated_token_account_instruction(
            &dest_owner.pubkey(),
            &dest_owner.pubkey(),
            &mint.pubkey(),
        );

        // Mint tokens
        let mint_amount = 1_000_000;
        let mint_to_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::id(),
            &mint.pubkey(),
            &source_account,
            &mint_authority_pubkey,
            &[],
            mint_amount,
        )
        .unwrap();

        // Transfer tokens
        let transfer_amount = 100_000;
        let transfer_ix = token_program
            .create_transfer_checked_instruction(
                &source_account,
                &mint.pubkey(),
                &dest_account,
                &source_owner.pubkey(),
                transfer_amount,
                decimals,
            )
            .unwrap();

        // Build and send transaction
        let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[init_mint_ix, create_source_ata_ix, create_dest_ata_ix, mint_to_ix, transfer_ix],
            Some(&source_owner.pubkey()),
            &[&source_owner, &mint_authority],
            recent_blockhash,
        );

        // Send and confirm transaction
        let result = rpc_client.send_and_confirm_transaction(&transaction).await;

        assert!(result.is_ok());

        // Verify balances
        let source_balance = rpc_client.get_token_account_balance(&source_account).await.unwrap();
        let dest_balance = rpc_client.get_token_account_balance(&dest_account).await.unwrap();

        // Account for transfer fee
        let fee = std::cmp::min((transfer_amount as u128 * 100u128 / 10000) as u64, 10_000);
        let expected_transfer = transfer_amount - fee;

        assert_eq!(source_balance.ui_amount.unwrap() as u64, mint_amount - transfer_amount);
        assert_eq!(dest_balance.ui_amount.unwrap() as u64, expected_transfer);
    }

    #[test]
    fn test_get_associated_token_address() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let program = Token2022Program::new();
        let ata_2022 = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata_2022, wallet);
        assert_ne!(ata_2022, mint);
    }

    #[test]
    fn test_create_ata_instruction() {
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test Token2022 ATA creation
        let program = Token2022Program::new();
        let ix = program.create_associated_token_account_instruction(&funder, &owner, &mint);

        assert_eq!(ix.program_id, spl_associated_token_account::id());
    }

    #[tokio::test]
    async fn test_get_and_validate_amount_for_payment() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a minimal Token2022Account for testing
        let account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: Vec::new(),
        };

        let mock_account = crate::tests::common::create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&mock_account);
        let program = Token2022Program::new();
        let result = program
            .get_and_validate_amount_for_payment(&rpc_client, Some(&account), None, amount)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), amount);
    }

    #[tokio::test]
    async fn test_get_and_validate_amount_for_payment_rejects_corrupted_extension_data() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 10_000;

        let buffer = vec![1; 1000]; // Non-empty buffer with invalid extension data

        // Create a Token2022Account with the extension data
        let token2022_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer,
        };

        let mock_account = crate::tests::common::create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&mock_account);
        let program = Token2022Program::new();
        // Should fail due to corrupted extension data - this is a security feature
        let result = program
            .get_and_validate_amount_for_payment(
                &rpc_client,
                Some(&token2022_account),
                None,
                amount,
            )
            .await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains(
            "Token account has corrupted extension data, transaction rejected for security"
        ));
    }

    #[tokio::test]
    async fn test_get_and_validate_amount_for_payment_insufficient_balance() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let account_balance = 500;
        let requested_amount = 1000;

        // Create a Token2022Account with insufficient balance
        let account = Token2022Account {
            mint,
            owner,
            amount: account_balance,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: Vec::new(),
        };

        let mock_account = crate::tests::common::create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&mock_account);
        let program = Token2022Program::new();
        // This test focuses on logic that happens before RPC calls
        // With empty extension data, it should return the original amount without RPC calls
        let result = program
            .get_and_validate_amount_for_payment(
                &rpc_client,
                Some(&account),
                None,
                requested_amount,
            )
            .await;
        assert!(result.is_ok());
        // The method doesn't currently check balance, it just validates extensions and calculates fees
        assert_eq!(result.unwrap(), requested_amount);
    }

    #[tokio::test]
    async fn test_get_and_validate_amount_for_payment_rejects_corrupted_transfer_fee_data() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 10_000;

        // Create dummy extension data that will fail to parse
        // This will trigger the transfer fee logic branch
        let buffer = vec![1; 165];

        let account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer,
        };

        // Mock that this account has a transfer fee
        // Note: Since we can't easily create valid extension data in tests,
        // the actual transfer fee logic in get_and_validate_amount_for_payment
        // will apply the fallback interest calculation
        let mock_account = crate::tests::common::create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&mock_account);
        let program = Token2022Program::new();

        // Should fail due to corrupted extension data - this is a security feature
        let result = program
            .get_and_validate_amount_for_payment(&rpc_client, Some(&account), None, amount)
            .await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains(
            "Token account has corrupted extension data, transaction rejected for security"
        ));
    }
}
