use super::trait_def::TokenProgram;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;
use spl_associated_token_account::{get_associated_token_address, create_associated_token_account};
use spl_token::instruction as token_instruction;
use crate::error::KoraError;

pub struct TokenkegProgram;

impl TokenProgram for TokenkegProgram {
    fn program_id(&self) -> Pubkey {
        spl_token::id()
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, KoraError> {
        Ok(token_instruction::transfer(
            &self.program_id(),
            source,
            destination,
            authority,
            &[],
            amount,
        )?)
    }

    fn create_close_account_instruction(
        &self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction, KoraError> {
        Ok(token_instruction::close_account(
            &self.program_id(),
            account,
            destination,
            authority,
            &[],
        )?)
    }

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, KoraError> {
        Ok(token_instruction::initialize_account(
            &self.program_id(),
            account,
            mint,
            owner,
        )?)
    }

    fn get_associated_token_address(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Pubkey {
        get_associated_token_address(wallet, mint)
    }

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, KoraError> {
        Ok(create_associated_token_account(
            funding_account,
            wallet,
            mint,
        ))
    }
}
