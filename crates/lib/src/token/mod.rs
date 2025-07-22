use crate::error::KoraError;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

mod interface;
mod token;
mod token22;

#[cfg(test)]
mod tests;

pub use interface::{TokenInterface, TokenState};
pub use token::{TokenAccount, TokenProgram};
pub use token22::{Token2022Account, Token2022Program};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Spl,
    Token2022,
}

impl TokenType {
    pub fn program_id(&self, token_interface: &impl TokenInterface) -> Pubkey {
        match self {
            TokenType::Spl => token_interface.program_id(),
            TokenType::Token2022 => token_interface.program_id(),
        }
    }
}

pub fn check_valid_tokens(tokens: &[String]) -> Result<Vec<Pubkey>, KoraError> {
    tokens
        .iter()
        .map(|token| {
            Pubkey::from_str(token)
                .map_err(|_| KoraError::ValidationError(format!("Invalid token address: {token}")))
        })
        .collect()
}
