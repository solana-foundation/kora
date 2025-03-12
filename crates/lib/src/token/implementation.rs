use super::{
    interface::{TokenInterface, TokenState},
    TokenType,
};
use async_trait::async_trait;
use solana_sdk::{instruction::Instruction, program_pack::Pack, pubkey::Pubkey};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token::state::{Account as TokenAccount, Mint};

pub struct TokenProgram {
    token_type: TokenType,
}

impl TokenProgram {
    pub fn new(token_type: TokenType) -> Self {
        Self { token_type }
    }
}

// Implement the interface version for spl_token::state::Account
impl TokenState for spl_token::state::Account {
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
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        self.token_type.program_id()
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

    fn decode_transfer_instruction(
        &self,
        data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let instruction = spl_token::instruction::TokenInstruction::unpack(data)?;
        match instruction {
            spl_token::instruction::TokenInstruction::Transfer { amount } => Ok(amount),
            _ => Err("Not a transfer instruction".into()),
        }
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = TokenAccount::unpack(data)?;
        Ok(Box::new(account))
    }

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let mint = Mint::unpack(mint_data)?;
        Ok(mint.decimals)
    }
}
