//! Core token types and implementations

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::{
    instruction::Instruction, lamports, program_error::ProgramError, program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Mint as TokenMint;
use spl_token_2022::state::Mint as Token22Mint;
use std::{ops::Deref, sync::Arc};

use crate::{
    token::{
        interface::TokenTrait,
        mint::TokenMintState,
        program::{Token22, TokenKeg},
    },
    transaction::TokenPriceInfo,
    KoraError,
};

/// Represents a token with its associated program implementation
#[derive(Debug)]
pub struct Token<T> {
    pub(crate) mint: TokenMintState,
    pub(crate) mint_address: Pubkey,
    pub(crate) price_info: TokenPriceInfo,
    pub(crate) token_program: T,
}

/// Enum representing either SPL Token or Token-2022 token
#[derive(Debug)]
pub enum TokenType {
    TokenKeg(Token<TokenKeg>),
    Token22(Token<Token22>),
}

impl<T> Token<T>
where
    T: TokenTrait,
{
    pub async fn try_from_mint(rpc: &RpcClient, mint: &Pubkey) -> Result<Self>
    where
        T: Default,
    {
        let mint_account = rpc.get_account(mint).await?;

        let mint_data = match mint_account.owner {
            owner if owner == spl_token::id() => {
                TokenMintState::TokenKeg(TokenMint::unpack(&mint_account.data)?)
            }
            owner if owner == spl_token_2022::id() => {
                TokenMintState::Token22(Token22Mint::unpack(&mint_account.data)?)
            }
            _ => anyhow::bail!("Invalid mint owner"),
        };

        // Verify the mint owner matches the token program
        let token_program = T::default();
        if mint_account.owner != token_program.id() {
            anyhow::bail!("Invalid mint owner for this token type");
        }

        Ok(Token {
            mint: mint_data,
            mint_address: *mint,
            price_info: TokenPriceInfo { price: 0.0 }, // need to implement this
            token_program,
        })
    }

    pub fn mint(&self) -> &TokenMintState {
        &self.mint
    }

    pub fn mint_address(&self) -> Pubkey {
        self.mint_address
    }

    pub fn price_info(&self) -> &TokenPriceInfo {
        &self.price_info
    }

    pub fn id(&self) -> Pubkey {
        self.token_program.id()
    }

    pub fn is_native(&self) -> bool {
        self.token_program.id() == spl_token::id()
    }
}

impl TokenType {
    pub async fn try_from_mint(rpc_client: &Arc<RpcClient>, token_mint: &Pubkey) -> Result<Self> {
        let mint_account = rpc_client
            .get_account(token_mint)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        match mint_account.owner {
            spl_token::ID => {
                let token = Token::<TokenKeg>::try_from_mint(rpc_client, token_mint).await?;
                Ok(TokenType::TokenKeg(token))
            }
            spl_token_2022::ID => {
                let token = Token::<Token22>::try_from_mint(rpc_client, token_mint).await?;
                Ok(TokenType::Token22(token))
            }
            _ => anyhow::bail!("Invalid mint owner"),
        }
    }

    pub fn mint(&self) -> &TokenMintState {
        match self {
            TokenType::TokenKeg(token) => token.mint(),
            TokenType::Token22(token) => token.mint(),
        }
    }

    pub fn price_info(&self) -> &TokenPriceInfo {
        match self {
            TokenType::TokenKeg(token) => token.price_info(),
            TokenType::Token22(token) => token.price_info(),
        }
    }

    pub fn token_program(&self) -> &dyn TokenTrait {
        match self {
            TokenType::TokenKeg(token) => &token.token_program,
            TokenType::Token22(token) => &token.token_program,
        }
    }

    pub fn native_transfer(&self, from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
        system_instruction::transfer(from, to, lamports)
    }
}

impl<T: TokenTrait> Deref for Token<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.token_program
    }
}

impl Deref for TokenType {
    type Target = dyn TokenTrait;

    fn deref(&self) -> &Self::Target {
        match self {
            TokenType::TokenKeg(token) => &token.token_program,
            TokenType::Token22(token) => &token.token_program,
        }
    }
}
