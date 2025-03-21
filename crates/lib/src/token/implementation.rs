use super::{
    interface::{TokenInterface, TokenState},
    TokenType,
};
use async_trait::async_trait;
use solana_program::{program_pack::Pack, pubkey::Pubkey as ProgramPubkey};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token::{
    self,
    state::{Account as TokenAccountState, Mint as MintState},
};

use spl_token::instruction::initialize_account;

// Define TokenAccount struct
pub struct TokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
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
            TokenType::Spl => Pubkey::new_from_array(spl_token::id().to_bytes()),
            TokenType::Token2022 => {
                todo!("Token2022 program ID logic")
            }
        }
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
        }))
    }

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let mint = MintState::unpack(mint_data)?;
        Ok(mint.decimals)
    }
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
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        self.get_program_id()
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
        use spl_token::instruction::TokenInstruction;
        let instruction = TokenInstruction::unpack(data)?;

        if let TokenInstruction::Transfer { amount } = instruction {
            Ok(amount)
        } else {
            Err("Invalid instruction type".into())
        }
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
        }))
    }

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let mint = MintState::unpack(mint_data)?;
        Ok(mint.decimals)
    }
}
