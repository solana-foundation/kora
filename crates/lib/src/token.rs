use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::id;
use std::str::FromStr;

use crate::error::KoraError;

mod base;
mod state;
mod program;

pub use base::TokenBase;
pub use program::{TokenProgram, TokenKeg};
pub use state::TokenState;

/// Represents supported token types in the system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    /// Standard SPL Token
    Spl,
    /// Token-2022 Program
    Token2022,
}

impl TokenType {
    /// Returns the program ID for this token type
    pub fn program_id(&self) -> Pubkey {
        match self {
            TokenType::Spl => spl_token::id(),
            TokenType::Token2022 => spl_token_2022::id(),
        }
    }
}

pub async fn check_valid_token(rpc_client: &RpcClient, token: &str) -> Result<(), KoraError> {
    let pubkey = Pubkey::from_str(token)
        .map_err(|e| KoraError::InternalServerError(format!("Invalid token address: {}", e)))?;

    // Check if the account exists and is a mint account
    match rpc_client.get_account(&pubkey).await {
        Ok(account) => {
            if account.owner == id() {
                Ok(())
            } else {
                Err(KoraError::InternalServerError(format!(
                    "Token {} is not a valid SPL token mint",
                    token
                )))
            }
        }
        Err(e) => {
            Err(KoraError::InternalServerError(format!("Token {} does not exist: {}", token, e)))
        }
    }
}

pub async fn check_valid_tokens(
    rpc_client: &RpcClient,
    tokens: &[String],
) -> Result<(), KoraError> {
    for token in tokens {
        check_valid_token(rpc_client, token).await?;
    }
    Ok(())
}
