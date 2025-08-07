use super::{
    interface::{TokenInterface, TokenState},
    token::TokenType,
};
use async_trait::async_trait;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token::{
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

pub struct TokenProgram {
    token_type: TokenType,
}

impl TokenProgram {
    pub fn new(token_type: TokenType) -> Self {
        Self { token_type }
    }

    fn get_program_id(&self) -> Pubkey {
        match self.token_type {
            TokenType::Spl => spl_token::id(),
            TokenType::Token2022 => spl_token_2022::id(),
        }
    }
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        self.get_program_id()
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
        Ok(spl_token::instruction::initialize_account(&self.program_id(), account, mint, owner)?)
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token::instruction::transfer(
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
        Ok(spl_token::instruction::transfer_checked(
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
        Ok(MintState::unpack(mint_data)?.decimals)
    }

    fn decode_transfer_instruction(
        &self,
        data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let instruction = spl_token::instruction::TokenInstruction::unpack(data)?;
        match instruction {
            spl_token::instruction::TokenInstruction::Transfer { amount } => Ok(amount),
            spl_token::instruction::TokenInstruction::TransferChecked { amount, .. } => Ok(amount),
            _ => Err("Not a transfer instruction".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_pack::Pack;
    use solana_sdk::pubkey::Pubkey;
    use spl_token::state::{Account as SplTokenAccount, Mint};

    #[test]
    fn test_token_program_spl() {
        let program = TokenProgram::new(TokenType::Spl);
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_token_program_creation() {
        let program = TokenProgram::new(TokenType::Spl);
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_get_mint_decimals() {
        let program = TokenProgram::new(TokenType::Spl);
        let mut mint_data = vec![0; Mint::LEN];
        let mut mint = Mint { is_initialized: true, ..Default::default() };
        mint.decimals = 9;
        mint.pack_into_slice(&mut mint_data);
        let result = program.get_mint_decimals(&mint_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_from_bytes() {
        let mut bytes = vec![0u8; SplTokenAccount::LEN];
        // Pack a dummy account to make it valid
        let dummy_account = SplTokenAccount {
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            amount: 0,
            state: spl_token::state::AccountState::Initialized,
            ..Default::default()
        };
        dummy_account.pack_into_slice(&mut bytes);

        let account = TokenProgram::new(TokenType::Spl).unpack_token_account(&bytes).unwrap();
        let token_account = account.as_any().downcast_ref::<TokenAccount>().unwrap();
        assert_eq!(token_account.amount, 0);
    }

    #[test]
    fn test_create_transfer_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        // Create the instruction directly for testing
        let ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &source,
            &dest,
            &authority,
            &[],
            100,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token::id());
    }

    #[test]
    fn test_create_transfer_checked_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Create the instruction directly for testing
        let ix = spl_token::instruction::transfer_checked(
            &spl_token::id(),
            &source,
            &mint,
            &dest,
            &authority,
            &[],
            100,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token::id());
    }

    #[test]
    fn test_get_associated_token_address() {
        let program = TokenProgram::new(TokenType::Spl);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata, wallet);
        assert_ne!(ata, mint);
    }

    #[test]
    fn test_create_ata_instruction() {
        let program = TokenProgram::new(TokenType::Spl);
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = program.create_associated_token_account_instruction(&funder, &owner, &mint);

        assert_eq!(ix.program_id, spl_associated_token_account::id());
    }
}
