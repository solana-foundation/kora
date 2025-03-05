use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::str::FromStr;
use crate::token::{SPL_TOKEN_PROGRAM_ID, TokenBase};

#[derive(Debug)]
pub enum TokenInstruction {
    InitializeAccount {
        owner: Pubkey,
    },
    Transfer {
        amount: u64,
    },
    TransferChecked {
        amount: u64,
        decimals: u8,
    },
}

impl TokenInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(9);
        match self {
            Self::InitializeAccount { owner: _ } => {
                buf.push(1); // Instruction index for InitializeAccount
            }
            Self::Transfer { amount } => {
                buf.push(3); // Instruction index for Transfer
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::TransferChecked { amount, decimals } => {
                buf.push(12); // Instruction index for TransferChecked
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(*decimals);
            }
        }
        buf
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            3 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::Transfer { amount }
            }
            12 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                let decimals = *rest.get(8).ok_or(ProgramError::InvalidInstructionData)?;
                Self::TransferChecked { amount, decimals }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

/// Represents a token program implementation
pub trait TokenProgram: TokenBase {}

/// Implementation for SPL Token program
#[derive(Debug, Default)]
pub struct TokenKeg;

impl TokenBase for TokenKeg {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap()
    }

    fn initialize_account(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: self.program_id(),
            accounts: vec![
                AccountMeta::new(*account, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(*owner, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::rent::id(), false),
            ],
            data: TokenInstruction::InitializeAccount { owner: *owner }.pack(),
        })
    }

    fn transfer(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
    ) -> Result<Instruction, ProgramError> {
        let mut accounts = vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ];
        for signer in signers {
            accounts.push(AccountMeta::new_readonly(**signer, true));
        }

        Ok(Instruction {
            program_id: self.program_id(),
            accounts,
            data: TokenInstruction::Transfer { amount }.pack(),
        })
    }

    fn transfer_checked(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        signers: &[&Pubkey],
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, ProgramError> {
        let mut accounts = vec![
            AccountMeta::new(*source, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ];
        for signer in signers {
            accounts.push(AccountMeta::new_readonly(**signer, true));
        }

        Ok(Instruction {
            program_id: self.program_id(),
            accounts,
            data: TokenInstruction::TransferChecked { amount, decimals }.pack(),
        })
    }

    fn decode_transfer_instruction(&self, data: &[u8]) -> Result<u64, ProgramError> {
        match TokenInstruction::unpack(data)? {
            TokenInstruction::Transfer { amount } => Ok(amount),
            TokenInstruction::TransferChecked { amount, .. } => Ok(amount),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

impl TokenProgram for TokenKeg {} 