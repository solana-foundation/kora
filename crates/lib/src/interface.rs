use anyhow::{Result, anyhow};
use solana_program::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_program,
};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAmount(pub u64);

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Invalid token account")]
    InvalidTokenAccount,
    #[error("Invalid mint")]
    InvalidMint,
    #[error("Amount overflow")]
    AmountOverflow,
    #[error("Program error: {0}")]
    ProgramError(String),
}

pub trait TokenInterface: Send + Sync + Debug {
    fn create_transfer_instruction(
        &self,
        from: &Pubkey,
        to: &Pubkey,
        authority: &Pubkey,
        amount: TokenAmount,
    ) -> Result<Instruction>;

    fn get_associated_token_address(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey>;

    fn create_associated_token_account_instruction(
        &self,
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction>;

    fn create_close_account_instruction(
        &self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction>;

    fn is_valid_token_account(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
    ) -> Result<bool>;

    fn program_id(&self) -> Pubkey;

    fn get_min_balance_for_rent_exemption(&self) -> Result<u64>;

    fn is_supported_mint(&self, mint: &Pubkey) -> bool;
}

#[derive(Debug, Clone)]
pub struct TokenKeg {
    program_id: Pubkey,
    supported_mints: Vec<Pubkey>,
}

impl TokenKeg {
    pub fn new(program_id: Pubkey, supported_mints: Vec<Pubkey>) -> Self {
        Self { 
            program_id,
            supported_mints,
        }
    }

    pub fn default() -> Self {
        Self {
            program_id: spl_token::id(),
            supported_mints: Vec::new(),
        }
    }
}

impl TokenInterface for TokenKeg {
    fn create_transfer_instruction(
        &self,
        from: &Pubkey,
        to: &Pubkey,
        authority: &Pubkey,
        amount: TokenAmount,
    ) -> Result<Instruction> {
        if !self.is_valid_token_account(from, &Pubkey::default())? {
            return Err(anyhow!(TokenError::InvalidTokenAccount));
        }

        Ok(spl_token::instruction::transfer(
            &self.program_id,
            from,
            to,
            authority,
            &[],
            amount.0,
        )?)
    }

    fn get_associated_token_address(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey> {
        if !self.is_supported_mint(mint) {
            return Err(anyhow!(TokenError::InvalidMint));
        }

        Ok(spl_associated_token_account::get_associated_token_address(
            wallet,
            mint,
        ))
    }

    fn create_associated_token_account_instruction(
        &self,
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction> {
        if !self.is_supported_mint(mint) {
            return Err(anyhow!(TokenError::InvalidMint));
        }

        Ok(spl_associated_token_account::instruction::create_associated_token_account(
            payer,
            wallet,
            mint,
            &self.program_id,
        ))
    }

    fn create_close_account_instruction(
        &self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction> {
        Ok(spl_token::instruction::close_account(
            &self.program_id,
            account,
            destination,
            authority,
            &[],
        )?)
    }

    fn is_valid_token_account(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
    ) -> Result<bool> {
        if !self.is_supported_mint(mint) {
            return Ok(false);
        }
        
        todo!("Implement proper token account validation")
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn get_min_balance_for_rent_exemption(&self) -> Result<u64> {
        todo!("Implement rent exemption calculation")
    }

    fn is_supported_mint(&self, mint: &Pubkey) -> bool {
        self.supported_mints.contains(mint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_token_keg() -> TokenKeg {
        let test_mint = Pubkey::new_unique();
        TokenKeg::new(spl_token::id(), vec![test_mint])
    }

    #[test]
    fn test_tokenkeg_new() {
        let program_id = system_program::id();
        let supported_mints = vec![Pubkey::new_unique()];
        let token_keg = TokenKeg::new(program_id, supported_mints.clone());
        assert_eq!(token_keg.program_id(), program_id);
        assert_eq!(token_keg.supported_mints, supported_mints);
    }

    #[test]
    fn test_supported_mint() {
        let test_mint = Pubkey::new_unique();
        let token_keg = TokenKeg::new(spl_token::id(), vec![test_mint]);
        
        assert!(token_keg.is_supported_mint(&test_mint));
        assert!(!token_keg.is_supported_mint(&Pubkey::new_unique()));
    }

    #[test]
    fn test_transfer_instruction_validation() {
        let token_keg = create_test_token_keg();
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = TokenAmount(100);

        let result = token_keg.create_transfer_instruction(&from, &to, &authority, amount);
        assert!(result.is_err() || std::thread::panicking());
    }
}
