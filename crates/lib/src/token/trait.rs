use solana_sdk::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;
use crate::error::KoraError;

pub trait TokenProgram {
    fn program_id(&self) -> Pubkey;
    
    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, KoraError>;

    fn create_close_account_instruction(
        &self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<Instruction, KoraError>;

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, KoraError>;

    fn get_associated_token_address(
        &self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Pubkey;

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, KoraError>;
}
