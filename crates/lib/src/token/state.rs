use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;
use spl_token_2022::state::Account as Token2022Account;

use crate::token::TokenType;

/// Represents the state of a token account
#[derive(Debug)]
pub enum TokenState {
    /// Standard SPL Token account
    Spl(TokenAccount),
    /// Token-2022 account
    Token2022(Token2022Account),
}

impl TokenState {
    /// Get the mint address for this token account
    pub fn mint(&self) -> Pubkey {
        match self {
            TokenState::Spl(account) => account.mint,
            TokenState::Token2022(account) => account.mint,
        }
    }

    /// Get the owner of this token account
    pub fn owner(&self) -> Pubkey {
        match self {
            TokenState::Spl(account) => account.owner,
            TokenState::Token2022(account) => account.owner,
        }
    }

    /// Get the token type for this account
    pub fn token_type(&self) -> TokenType {
        match self {
            TokenState::Spl(_) => TokenType::Spl,
            TokenState::Token2022(_) => TokenType::Token2022,
        }
    }
} 