//! Defines common interface for token program operations.
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::error::KoraError;
use solana_sdk::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey};

mod mint;
mod program;
mod types;

pub use mint::{MintStateInterface, TokenMintState};
pub use program::{Token22, TokenKeg};
pub use types::{Token, TokenType};

/// Common interface for token program operations.
pub trait TokenTrait: Send + Sync {
    /// Returns the program ID
    fn id(&self) -> Pubkey;

    fn get_associated_token_address(
        &self,
        wallet_address: &Pubkey,
        token_mint_address: &Pubkey,
    ) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(
            wallet_address,
            token_mint_address,
        )
    }

    fn create_associated_token_account(
        &self,
        funding_address: &Pubkey,
        wallet_address: &Pubkey,
        token_mint_address: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Instruction {
        spl_associated_token_account::instruction::create_associated_token_account(
            funding_address,
            wallet_address,
            token_mint_address,
            token_program_id,
        )
    }

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

    #[allow(clippy::too_many_arguments)]
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

pub async fn check_valid_tokens(
    rpc_client: &Arc<RpcClient>,
    tokens: &[String],
) -> Result<(), KoraError> {
    for token in tokens {
        let pk = token
            .parse::<Pubkey>()
            .map_err(|_| KoraError::ValidationError("Could not parse token.".into()))?;

        TokenType::try_from_mint(rpc_client, &pk).await?;
    }
    Ok(())
}
