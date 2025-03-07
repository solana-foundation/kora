//! Defines interfaces and types for token mint accounts.

use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_token::state::Mint as TokenMint;
use spl_token_2022::state::Mint as Token22Mint;
use std::ops::Deref;

/// Represents either a Token or Token-2022 mint account
#[derive(Debug)]
pub enum TokenMintState {
    TokenKeg(TokenMint),
    Token22(Token22Mint),
}

impl Deref for TokenMintState {
    type Target = dyn MintStateInterface;

    fn deref(&self) -> &Self::Target {
        match self {
            TokenMintState::TokenKeg(mint) => mint,
            TokenMintState::Token22(mint) => mint,
        }
    }
}

/// Common interface for mint account data
pub trait MintStateInterface {
    fn supply(&self) -> u64;
    fn decimals(&self) -> u8;
    fn mint_authority(&self) -> Option<Pubkey>;
    fn freeze_authority(&self) -> Option<Pubkey>;
    fn is_initialized(&self) -> bool;
}

// Implement for both mint types
impl MintStateInterface for TokenMint {
    fn supply(&self) -> u64 {
        self.supply
    }
    fn decimals(&self) -> u8 {
        self.decimals
    }
    fn mint_authority(&self) -> Option<Pubkey> {
        self.mint_authority.into()
    }
    fn freeze_authority(&self) -> Option<Pubkey> {
        self.freeze_authority.into()
    }
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl MintStateInterface for Token22Mint {
    fn supply(&self) -> u64 {
        self.supply
    }
    fn decimals(&self) -> u8 {
        self.decimals
    }
    fn mint_authority(&self) -> Option<Pubkey> {
        self.mint_authority.into()
    }
    fn freeze_authority(&self) -> Option<Pubkey> {
        self.freeze_authority.into()
    }
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
