use async_trait::async_trait;
use mockall::automock;
use solana_sdk::{
    instruction::{CompiledInstruction, Instruction},
    pubkey::Pubkey,
};
use std::any::Any;

use crate::validator::transaction_validator::TransactionValidator;

pub trait TokenState: Any {
    fn mint(&self) -> Pubkey;
    fn owner(&self) -> Pubkey;
    fn amount(&self) -> u64;
    fn decimals(&self) -> u8;

    // Add method to support downcasting for Token2022 specific features
    fn as_any(&self) -> &dyn Any;
}

pub trait TokenMint: Any {
    fn address(&self) -> Pubkey;
    fn mint_authority(&self) -> Option<Pubkey>;
    fn supply(&self) -> u64;
    fn decimals(&self) -> u8;
    fn freeze_authority(&self) -> Option<Pubkey>;
    fn is_initialized(&self) -> bool;
    fn get_token_program(&self) -> Box<dyn TokenInterface>;

    // For downcasting to specific types
    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
#[automock]
pub trait TokenInterface: Send + Sync {
    fn program_id(&self) -> Pubkey;

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>;

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>>;

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>>;

    fn create_transfer_checked_instruction(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>>;

    fn get_associated_token_address(&self, wallet: &Pubkey, mint: &Pubkey) -> Pubkey;

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction;

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>;

    fn decode_transfer_instruction(
        &self,
        data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    fn process_spl_instruction(
        &self,
        instruction_data: &[u8],
        ix: &solana_sdk::instruction::CompiledInstruction,
        account_keys: &[solana_sdk::pubkey::Pubkey],
        fee_payer_pubkey: &solana_sdk::pubkey::Pubkey,
        command: SplInstructionCommand,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    fn get_and_validate_amount_for_payment(
        &self,
        token_account: &dyn TokenState,
        amount: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplInstructionType {
    Transfer,
    TransferChecked,
    Burn,
    BurnChecked,
    CloseAccount,
    Approve,
    ApproveChecked,
}

#[derive(Debug, Clone)]
pub struct ParsedSplInstruction {
    pub instruction_type: SplInstructionType,
    pub authority_index: usize,
    pub amount: Option<u64>,
    pub program_id: solana_sdk::pubkey::Pubkey,
}

#[derive(Debug, Clone)]
pub enum SplInstructionCommand {
    ValidateFeePayerPolicy { fee_payer_policy: crate::config::FeePayerPolicy },
}

impl SplInstructionCommand {
    pub fn execute(
        &self,
        parsed: &ParsedSplInstruction,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
        fee_payer_pubkey: &Pubkey,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            SplInstructionCommand::ValidateFeePayerPolicy { fee_payer_policy } => {
                let policy_flag = match parsed.instruction_type {
                    SplInstructionType::Transfer | SplInstructionType::TransferChecked => {
                        if parsed.program_id == spl_token_2022::id() {
                            fee_payer_policy.allow_token2022_transfers
                        } else {
                            fee_payer_policy.allow_spl_transfers
                        }
                    }
                    SplInstructionType::Burn | SplInstructionType::BurnChecked => {
                        fee_payer_policy.allow_burn
                    }
                    SplInstructionType::Approve | SplInstructionType::ApproveChecked => {
                        fee_payer_policy.allow_approve
                    }
                    SplInstructionType::CloseAccount => fee_payer_policy.allow_close_account,
                };

                TransactionValidator::check_fee_payer(
                    fee_payer_pubkey,
                    parsed.authority_index,
                    policy_flag,
                    ix,
                    account_keys,
                )
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }
    }
}
