use solana_sdk::{
    instruction::Instruction,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Base trait for token operations
#[async_trait::async_trait]
pub trait TokenBase: Send + Sync {
    /// Returns the program ID for this token implementation
    fn program_id(&self) -> Pubkey;

    /// Creates an instruction to initialize a new token account
    fn initialize_account(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, ProgramError>;

    /// Creates a transfer instruction
    fn transfer(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
    ) -> Result<Instruction, ProgramError>;

    /// Creates a checked transfer instruction with decimal verification
    fn transfer_checked(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError>;

    /// Decodes a transfer instruction to extract the amount
    fn decode_transfer_instruction(&self, data: &[u8]) -> Result<u64, ProgramError>;
} 