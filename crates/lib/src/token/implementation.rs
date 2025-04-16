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
use spl_token_2022;

use spl_token::instruction::initialize_account;

// Define TokenAccount struct
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
    pub decimals: u8,
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
            TokenType::Token2022 => Pubkey::new_from_array(spl_token_2022::id().to_bytes()),
        }
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = match self.token_type {
            TokenType::Spl => {
                let acc = TokenAccountState::unpack(data)?;
                TokenAccount {
                    mint: acc.mint,
                    owner: acc.owner,
                    amount: acc.amount,
                    delegate: acc.delegate,
                    state: acc.state,
                    is_native: acc.is_native,
                    delegated_amount: acc.delegated_amount,
                    close_authority: acc.close_authority,
                    decimals: 0, // Will be set from mint
                }
            }
            TokenType::Token2022 => {
                let acc = spl_token_2022::state::Account::unpack(data)?;
                TokenAccount {
                    mint: acc.mint,
                    owner: acc.owner,
                    amount: acc.amount,
                    delegate: acc.delegate,
                    state: acc.state,
                    is_native: acc.is_native,
                    delegated_amount: acc.delegated_amount,
                    close_authority: acc.close_authority,
                    decimals: 0, // Will be set from mint
                }
            }
        };
        Ok(Box::new(account))
    }

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let mint = match self.token_type {
            TokenType::Spl => MintState::unpack(mint_data)?,
            TokenType::Token2022 => spl_token_2022::state::Mint::unpack(mint_data)?,
        };
        Ok(mint.decimals)
    }

    fn validate_token_account(&self, account: &TokenAccount) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check account state
        if account.state != spl_token::state::AccountState::Initialized as u8 {
            return Err("Token account not initialized".into());
        }

        // Check if account is frozen
        if account.state == spl_token::state::AccountState::Frozen as u8 {
            return Err("Token account is frozen".into());
        }

        Ok(())
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
        self.decimals
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
        match self.token_type {
            TokenType::Spl => Ok(spl_token::instruction::initialize_account(
                &self.program_id(),
                account,
                mint,
                owner,
            )?),
            TokenType::Token2022 => Ok(spl_token_2022::instruction::initialize_account(
                &self.program_id(),
                account,
                mint,
                owner,
            )?),
        }
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        match self.token_type {
            TokenType::Spl => Ok(spl_token::instruction::transfer(
                &self.program_id(),
                source,
                destination,
                authority,
                &[],
                amount,
            )?),
            TokenType::Token2022 => Ok(spl_token_2022::instruction::transfer(
                &self.program_id(),
                source,
                destination,
                authority,
                &[],
                amount,
            )?),
        }
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
        // Always prefer transfer_checked over regular transfer for safety
        match self.token_type {
            TokenType::Spl => Ok(spl_token::instruction::transfer_checked(
                &self.program_id(),
                source,
                mint,
                destination,
                authority,
                &[],
                amount,
                decimals,
            )?),
            TokenType::Token2022 => Ok(spl_token_2022::instruction::transfer_checked(
                &self.program_id(),
                source,
                mint,
                destination,
                authority,
                &[],
                amount,
                decimals,
            )?),
        }
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
        match self.token_type {
            TokenType::Spl => {
                use spl_token::instruction::TokenInstruction;
                let instruction = TokenInstruction::unpack(data)?;
                match instruction {
                    TokenInstruction::Transfer { amount } => Ok(amount),
                    TokenInstruction::TransferChecked { amount, .. } => Ok(amount),
                    _ => Err("Invalid instruction type".into()),
                }
            }
            TokenType::Token2022 => {
                use spl_token_2022::instruction::TokenInstruction;
                let instruction = TokenInstruction::unpack(data)?;
                match instruction {
                    TokenInstruction::Transfer { amount } => Ok(amount),
                    TokenInstruction::TransferChecked { amount, .. } => Ok(amount),
                    _ => Err("Invalid instruction type".into()),
                }
            }
        }
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = self.unpack_token_account(data)?;
        if let Some(token_account) = account.as_any().downcast_ref::<TokenAccount>() {
            self.validate_token_account(token_account)?;
        }
        Ok(account)
    }

    fn get_mint_decimals(
        &self,
        mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        self.get_mint_decimals(mint_data)
    }
}
