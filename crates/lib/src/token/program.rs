//! Implements token program interfaces for SPL Token and Token-2022

use crate::token::interface::TokenTrait;
use solana_sdk::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey};

/// SPL Token program implementation
#[derive(Debug, Default)]
pub struct TokenKeg;

/// Token-2022 program implementation
#[derive(Debug, Default)]
pub struct Token22;

impl TokenKeg {
    fn set_authority(
        &self,
        owned_pubkey: &Pubkey,
        new_authority_pubkey: Option<&Pubkey>,
        authority_type: spl_token::instruction::AuthorityType,
        owner_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
    ) -> Result<Instruction, ProgramError> {
        spl_token::instruction::set_authority(
            &self.id(),
            owned_pubkey,
            new_authority_pubkey,
            authority_type,
            owner_pubkey,
            signer_pubkeys,
        )
    }
}

impl Token22 {
    fn set_authority(
        &self,
        owned_pubkey: &Pubkey,
        new_authority_pubkey: Option<&Pubkey>,
        authority_type: spl_token_2022::instruction::AuthorityType,
        owner_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
    ) -> Result<Instruction, ProgramError> {
        spl_token_2022::instruction::set_authority(
            &self.id(),
            owned_pubkey,
            new_authority_pubkey,
            authority_type,
            owner_pubkey,
            signer_pubkeys,
        )
    }
}

impl TokenTrait for TokenKeg {
    fn id(&self) -> Pubkey {
        spl_token::id()
    }

    fn initialize_account(
        &self,
        account_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        spl_token::instruction::initialize_account(
            &self.id(),
            account_pubkey,
            mint_pubkey,
            owner_pubkey,
        )
    }

    fn transfer(
        &self,
        source_pubkey: &Pubkey,
        destination_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
    ) -> Result<Instruction, ProgramError> {
        spl_token::instruction::transfer(
            &self.id(),
            source_pubkey,
            destination_pubkey,
            authority_pubkey,
            signer_pubkeys,
            amount,
        )
    }

    fn transfer_checked(
        &self,
        source_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        destination_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError> {
        spl_token::instruction::transfer_checked(
            &self.id(),
            source_pubkey,
            mint_pubkey,
            destination_pubkey,
            authority_pubkey,
            signer_pubkeys,
            amount,
            decimals,
        )
    }
}

impl TokenTrait for Token22 {
    fn id(&self) -> Pubkey {
        spl_token_2022::id()
    }

    fn initialize_account(
        &self,
        account_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        spl_token_2022::instruction::initialize_account(
            &self.id(),
            account_pubkey,
            mint_pubkey,
            owner_pubkey,
        )
    }

    fn transfer(
        &self,
        _source_pubkey: &Pubkey,
        _destination_pubkey: &Pubkey,
        _authority_pubkey: &Pubkey,
        _signer_pubkeys: &[&Pubkey],
        _amount: u64,
    ) -> Result<Instruction, ProgramError> {
        unimplemented!("DEPRECATED")
    }

    fn transfer_checked(
        &self,
        source_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        destination_pubkey: &Pubkey,
        authority_pubkey: &Pubkey,
        signer_pubkeys: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError> {
        spl_token_2022::instruction::transfer_checked(
            &self.id(),
            source_pubkey,
            mint_pubkey,
            destination_pubkey,
            authority_pubkey,
            signer_pubkeys,
            amount,
            decimals,
        )
    }
}
