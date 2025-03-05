use solana_sdk::{program_error::ProgramError, pubkey::Pubkey, program_pack::Pack};
use std::convert::TryFrom;

use crate::token::TokenType;

/// Common token account data structure
#[derive(Debug, Clone)]
pub struct TokenAccountData {
    /// The mint that this account holds tokens for
    pub mint: Pubkey,
    /// The owner of this account
    pub owner: Pubkey,
    /// The amount of tokens this account holds
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_native.is_some, this is a native token, and the value logs the rent-exempt reserve
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account
    pub close_authority: Option<Pubkey>,
}

/// Account state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccountState {
    /// Account is not yet initialized
    Uninitialized,
    /// Account is initialized; the account owner and/or delegate may perform permitted operations
    /// on this account
    Initialized,
    /// Account has been frozen by the mint freeze authority. Neither the account owner nor
    /// the delegate are able to perform operations on this account.
    Frozen,
}

/// Represents the state of a token account
#[derive(Debug)]
pub enum TokenState {
    /// Standard SPL Token account
    Spl(TokenAccountData),
    /// Token-2022 account
    Token2022(TokenAccountData),
}

impl TryFrom<&[u8]> for TokenAccountData {
    type Error = ProgramError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() < 165 {
            return Err(ProgramError::InvalidAccountData);
        }

        let mint = Pubkey::new_from_array(<[u8; 32]>::try_from(&input[0..32]).unwrap());
        let owner = Pubkey::new_from_array(<[u8; 32]>::try_from(&input[32..64]).unwrap());
        let amount = u64::from_le_bytes(<[u8; 8]>::try_from(&input[64..72]).unwrap());
        
        let delegate = if input[72] == 0 {
            None
        } else {
            Some(Pubkey::new_from_array(<[u8; 32]>::try_from(&input[73..105]).unwrap()))
        };

        let state = match input[105] {
            0 => AccountState::Uninitialized,
            1 => AccountState::Initialized,
            2 => AccountState::Frozen,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        let is_native = if input[106] == 0 {
            None
        } else {
            Some(u64::from_le_bytes(<[u8; 8]>::try_from(&input[107..115]).unwrap()))
        };

        let delegated_amount = u64::from_le_bytes(<[u8; 8]>::try_from(&input[115..123]).unwrap());

        let close_authority = if input[123] == 0 {
            None
        } else {
            Some(Pubkey::new_from_array(<[u8; 32]>::try_from(&input[124..156]).unwrap()))
        };

        Ok(TokenAccountData {
            mint,
            owner,
            amount,
            delegate,
            state,
            is_native,
            delegated_amount,
            close_authority,
        })
    }
}

impl TokenState {
    /// Try to deserialize token account data
    pub fn try_from_slice(data: &[u8]) -> Result<Self, ProgramError> {
        let account_data = TokenAccountData::try_from(data)?;
        // For now, we'll assume it's an SPL token if the data length matches
        // In a real implementation, we'd need more sophisticated detection
        if data.len() == 165 {
            Ok(TokenState::Spl(account_data))
        } else {
            Ok(TokenState::Token2022(account_data))
        }
    }

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

    /// Get the amount for this token account
    pub fn amount(&self) -> u64 {
        match self {
            TokenState::Spl(account) => account.amount,
            TokenState::Token2022(account) => account.amount,
        }
    }

    /// Get the token type for this account
    pub fn token_type(&self) -> TokenType {
        match self {
            TokenState::Spl(_) => TokenType::Spl,
            TokenState::Token2022(_) => TokenType::Token2022,
        }
    }

    /// Get the decimals for this token account
    pub fn decimals(&self) -> u8 {
        8 // Standard SPL token decimals
    }
} 