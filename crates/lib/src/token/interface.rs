//! Defines common interface for token program operations.

use solana_sdk::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey};

/// Common interface for token program operations.
pub trait TokenTrait: Send + Sync {
    /// Returns the program ID
    fn id(&self) -> Pubkey;

    /// Creates an instruction to initialize a new token account
    fn initialize_account(
        &self,
        account_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Result<Instruction, ProgramError>;

    /// Creates a transfer instruction
    fn transfer(
        &self,
        source_pubkey: &Pubkey,
        destination_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
    ) -> Result<Instruction, ProgramError>;

    /// Creates a checked transfer instruction
    fn transfer_checked(
        &self,
        source_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        destination_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError>;
}
