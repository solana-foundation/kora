use super::{
    interface::{TokenInterface, TokenState},
    TokenType,
};
use async_trait::async_trait;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_pod::bytemuck;
use spl_token_2022::{
    extension::{
        confidential_transfer::ConfidentialTransferAccount,
        confidential_transfer_fee::ConfidentialTransferFeeConfig, cpi_guard::CpiGuard,
        default_account_state::DefaultAccountState, group_member_pointer::GroupMemberPointer,
        group_pointer::GroupPointer, immutable_owner::ImmutableOwner,
        interest_bearing_mint::InterestBearingConfig, memo_transfer::MemoTransfer,
        metadata_pointer::MetadataPointer, mint_close_authority::MintCloseAuthority,
        non_transferable::NonTransferable, pausable::PausableAccount,
        permanent_delegate::PermanentDelegate, transfer_fee::TransferFeeConfig,
        transfer_hook::TransferHook, BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    instruction,
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
    pub fn get_transfer_fee(&self) -> Option<TransferFeeConfig> {
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
        self.has_extension(ExtensionType::NonTransferable)
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

pub struct Token2022Program;

impl Token2022Program {
    pub fn new() -> Self {
        Self
    }

    fn get_program_id(&self) -> Pubkey {
        spl_token_2022::id()
    }
}

#[async_trait]
impl TokenInterface for Token2022Program {
    fn program_id(&self) -> Pubkey {
        self.get_program_id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = StateWithExtensions::<Token2022AccountState>::unpack(data)?;
        let base = account.base;

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
            extension_data: data.to_vec(),
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

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let mint = StateWithExtensions::<Token2022MintState>::unpack(mint_data)?;
        Ok(mint.base.decimals)
    }

    fn decode_transfer_instruction(
        &self,
        data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let instruction = instruction::TokenInstruction::unpack(data)?;
        match instruction {
            instruction::TokenInstruction::Transfer { amount } => Ok(amount),
            instruction::TokenInstruction::TransferChecked { amount, .. } => Ok(amount),
            _ => Err("Not a transfer instruction".into()),
        }
    }
}
