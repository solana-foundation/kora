use solana_sdk::{
    instruction::Instruction,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::token::base::TokenBase;

/// Represents a token program implementation
pub trait TokenProgram: TokenBase {}

/// Implementation for SPL Token program
#[derive(Debug, Default)]
pub struct TokenKeg;

impl TokenBase for TokenKeg {
    fn program_id(&self) -> Pubkey {
        spl_token::id()
    }

    fn initialize_account(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        Ok(spl_token::instruction::initialize_account(
            &self.program_id(),
            account,
            mint,
            owner,
        )?)
    }

    fn transfer(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
    ) -> Result<Instruction, ProgramError> {
        Ok(spl_token::instruction::transfer(
            &self.program_id(),
            source,
            destination,
            authority,
            signers,
            amount,
        )?)
    }

    fn transfer_checked(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError> {
        Ok(spl_token::instruction::transfer_checked(
            &self.program_id(),
            source,
            mint,
            destination,
            authority,
            signers,
            amount,
            decimals,
        )?)
    }
}

impl TokenProgram for TokenKeg {} 