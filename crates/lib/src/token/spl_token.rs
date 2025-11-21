use crate::token::interface::TokenMint;

use super::interface::{TokenInterface, TokenState};
use async_trait::async_trait;
use solana_program::pubkey::Pubkey;
use solana_sdk::{instruction::Instruction, program_pack::Pack};
use spl_associated_token_account_interface::{
    address::get_associated_token_address_with_program_id,
    instruction::create_associated_token_account,
};
use spl_token_interface::{
    self,
    state::{Account as TokenAccountState, AccountState, Mint as MintState},
};

#[derive(Debug)]
pub struct TokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: u8,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
}

impl TokenState for TokenAccount {
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

#[derive(Debug)]
pub struct SplMint {
    pub mint: Pubkey,
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}

impl TokenMint for SplMint {
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
        Box::new(TokenProgram::new())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct TokenProgram;

impl Default for TokenProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenProgram {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        spl_token_interface::id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = TokenAccountState::unpack(data)?;

        Ok(Box::new(TokenAccount {
            mint: account.mint,
            owner: account.owner,
            amount: account.amount,
            delegate: account.delegate.into(),
            state: match account.state {
                AccountState::Uninitialized => 0,
                AccountState::Initialized => 1,
                AccountState::Frozen => 2,
            },
            is_native: account.is_native.into(),
            delegated_amount: account.delegated_amount,
            close_authority: account.close_authority.into(),
        }))
    }

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token_interface::instruction::initialize_account(
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
        Ok(spl_token_interface::instruction::transfer(
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
        Ok(spl_token_interface::instruction::transfer_checked(
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
        let mint_state = MintState::unpack(mint_data)?;

        Ok(Box::new(SplMint {
            mint: *mint,
            mint_authority: mint_state.mint_authority.into(),
            supply: mint_state.supply,
            decimals: mint_state.decimals,
            is_initialized: mint_state.is_initialized,
            freeze_authority: mint_state.freeze_authority.into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::common::{MintAccountMockBuilder, TokenAccountMockBuilder};

    use super::*;
    use solana_program::program_pack::Pack;
    use solana_sdk::pubkey::Pubkey;
    use spl_token_interface::state::{Account as SplTokenAccount, AccountState};

    #[test]
    fn test_token_program_creation_and_program_id() {
        let program = TokenProgram::new();
        assert_eq!(program.program_id(), spl_token_interface::id());
    }

    #[test]
    fn test_unpack_token_account_success() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let close_authority = Pubkey::new_unique();
        let amount = 1000000;
        let delegated_amount = 500000;
        let is_native = Some(2039280u64);

        let account = TokenAccountMockBuilder::new()
            .with_mint(&mint)
            .with_owner(&owner)
            .with_amount(amount)
            .with_state(AccountState::Initialized)
            .with_delegate(Some(delegate))
            .with_native(is_native)
            .with_delegated_amount(delegated_amount)
            .with_close_authority(Some(close_authority))
            .build();

        let program = TokenProgram::new();
        let result = program.unpack_token_account(&account.data);
        assert!(result.is_ok());

        let token_state = result.unwrap();
        let token_account = token_state.as_any().downcast_ref::<TokenAccount>().unwrap();

        assert_eq!(token_account.mint, mint);
        assert_eq!(token_account.owner, owner);
        assert_eq!(token_account.amount, amount);
        assert_eq!(token_account.delegate, Some(delegate));
        assert_eq!(token_account.state, 1); // AccountState::Initialized = 1
        assert_eq!(token_account.is_native, is_native);
        assert_eq!(token_account.delegated_amount, delegated_amount);
        assert_eq!(token_account.close_authority, Some(close_authority));
    }

    #[test]
    fn test_unpack_token_account_invalid_data() {
        let program = TokenProgram::new();

        // Test with empty data
        let result = program.unpack_token_account(&[]);
        assert!(result.is_err());

        // Test with insufficient data
        let short_data = vec![0u8; 10];
        let result = program.unpack_token_account(&short_data);
        assert!(result.is_err());

        // Test with corrupted data
        let mut corrupted_data = vec![0xFFu8; SplTokenAccount::LEN];
        corrupted_data[0] = 0xFF; // Invalid mint pubkey start
        let result = program.unpack_token_account(&corrupted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_unpack_mint_success() {
        let mint_pubkey = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        let freeze_authority = Pubkey::new_unique();
        let supply = 1000000000;
        let decimals = 6;

        let account = MintAccountMockBuilder::new()
            .with_mint_authority(Some(mint_authority))
            .with_supply(supply)
            .with_decimals(decimals)
            .with_initialized(true)
            .with_freeze_authority(Some(freeze_authority))
            .build();

        let program = TokenProgram::new();
        let result = program.unpack_mint(&mint_pubkey, &account.data);
        assert!(result.is_ok());

        let token_mint = result.unwrap();
        let spl_mint = token_mint.as_any().downcast_ref::<SplMint>().unwrap();

        assert_eq!(spl_mint.mint, mint_pubkey);
        assert_eq!(spl_mint.mint_authority, Some(mint_authority));
        assert_eq!(spl_mint.supply, supply);
        assert_eq!(spl_mint.decimals, decimals);
        assert!(spl_mint.is_initialized);
        assert_eq!(spl_mint.freeze_authority, Some(freeze_authority));
    }

    #[test]
    fn test_unpack_mint_with_none_authorities() {
        let mint_pubkey = Pubkey::new_unique();
        // Create initialized mint with None authorities (this is valid)
        let account = MintAccountMockBuilder::new()
            .with_mint_authority(None)
            .with_supply(0)
            .with_decimals(0)
            .with_initialized(true)
            .with_freeze_authority(None)
            .build();

        let program = TokenProgram::new();
        let result = program.unpack_mint(&mint_pubkey, &account.data).unwrap();
        let spl_mint = result.as_any().downcast_ref::<SplMint>().unwrap();

        assert_eq!(spl_mint.mint_authority, None);
        assert_eq!(spl_mint.freeze_authority, None);
        assert!(spl_mint.is_initialized); // Should be initialized to be valid
    }

    #[test]
    fn test_unpack_mint_invalid_data() {
        let mint_pubkey = Pubkey::new_unique();
        let program = TokenProgram::new();

        // Test with empty data
        let result = program.unpack_mint(&mint_pubkey, &[]);
        assert!(result.is_err());

        // Test with insufficient data
        let short_data = vec![0u8; 10];
        let result = program.unpack_mint(&mint_pubkey, &short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_initialize_account_instruction() {
        let program = TokenProgram::new();
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = program.create_initialize_account_instruction(&account, &mint, &owner);
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.program_id, spl_token_interface::id());
        assert_eq!(instruction.accounts.len(), 4); // account, mint, owner, rent sysvar
    }

    #[test]
    fn test_create_transfer_instruction() {
        let program = TokenProgram::new();
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 1000000;

        let result = program.create_transfer_instruction(&source, &destination, &authority, amount);
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.program_id, spl_token_interface::id());
        assert_eq!(instruction.accounts.len(), 3); // source, destination, authority
    }

    #[test]
    fn test_create_transfer_checked_instruction() {
        let program = TokenProgram::new();
        let source = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 1000000;
        let decimals = 6;

        let result = program.create_transfer_checked_instruction(
            &source,
            &mint,
            &destination,
            &authority,
            amount,
            decimals,
        );
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.program_id, spl_token_interface::id());
        assert_eq!(instruction.accounts.len(), 4); // source, mint, destination, authority
    }

    #[test]
    fn test_get_associated_token_address() {
        let program = TokenProgram::new();
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);

        let ata2 = get_associated_token_address_with_program_id(
            &wallet,
            &mint,
            &spl_token_interface::id(),
        );

        assert_eq!(ata, ata2);
    }

    #[test]
    fn test_create_associated_token_account_instruction() {
        let program = TokenProgram::new();
        let funding_account = Pubkey::new_unique();
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let instruction =
            program.create_associated_token_account_instruction(&funding_account, &wallet, &mint);

        assert_eq!(instruction.program_id, spl_associated_token_account_interface::program::id());
        assert_eq!(instruction.accounts.len(), 6); // funding, ata, wallet, mint, system_program, token_program
    }

    #[test]
    fn test_spl_mint_get_token_program() {
        let spl_mint = SplMint {
            mint: Pubkey::new_unique(),
            mint_authority: None,
            supply: 0,
            decimals: 0,
            is_initialized: false,
            freeze_authority: None,
        };

        let token_program = spl_mint.get_token_program();
        assert_eq!(token_program.program_id(), spl_token_interface::id());
    }

    #[test]
    fn test_spl_mint_as_any_downcasting() {
        let spl_mint = SplMint {
            mint: Pubkey::new_unique(),
            mint_authority: None,
            supply: 1000000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
        };

        let any_ref = spl_mint.as_any();
        assert!(any_ref.is::<SplMint>());

        let downcast_result = any_ref.downcast_ref::<SplMint>();
        assert!(downcast_result.is_some());
        assert_eq!(downcast_result.unwrap().supply, 1000000);
    }

    #[test]
    fn test_spl_mint_with_none_authorities() {
        let spl_mint = SplMint {
            mint: Pubkey::new_unique(),
            mint_authority: None,
            supply: 0,
            decimals: 0,
            is_initialized: false,
            freeze_authority: None,
        };

        assert_eq!(spl_mint.mint_authority(), None);
        assert_eq!(spl_mint.freeze_authority(), None);
        assert!(!spl_mint.is_initialized());
    }
}
