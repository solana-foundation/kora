use super::interface::{TokenInterface, TokenMint, TokenState};
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use spl_token_2022_interface::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    instruction::{initialize_account, transfer, transfer_checked},
    state::{Account, Mint},
};
use std::any::Any;

pub struct Token2022Program;

impl Token2022Program {
    pub fn new() -> Self {
        Self
    }
}

pub struct Token2022ProgramState(pub Account);

impl TokenState for Token2022ProgramState {
    fn mint(&self) -> Pubkey {
        self.0.mint
    }

    fn owner(&self) -> Pubkey {
        self.0.owner
    }

    fn amount(&self) -> u64 {
        self.0.amount
    }

    fn decimals(&self) -> u8 {
        0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Token2022ProgramMint {
    pub address: Pubkey,
    pub mint: Mint,
    pub transfer_fee_config: Option<TransferFeeConfig>,
}

impl Token2022ProgramMint {
    pub fn calculate_transfer_fee(&self, amount: u64, epoch: u64) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(config) = self.transfer_fee_config {
            let fee = config.calculate_epoch_fee(epoch, amount);
            Ok(fee.unwrap_or(0))
        } else {
            Ok(0)
        }
    }
}

impl TokenMint for Token2022ProgramMint {
    fn address(&self) -> Pubkey {
        self.address
    }

    fn mint_authority(&self) -> Option<Pubkey> {
        self.mint.mint_authority.into()
    }

    fn supply(&self) -> u64 {
        self.mint.supply
    }

    fn decimals(&self) -> u8 {
        self.mint.decimals
    }

    fn freeze_authority(&self) -> Option<Pubkey> {
        self.mint.freeze_authority.into()
    }

    fn is_initialized(&self) -> bool {
        self.mint.is_initialized
    }

    fn get_token_program(&self) -> Box<dyn TokenInterface> {
        Box::new(Token2022Program::new())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl TokenInterface for Token2022Program {
    fn program_id(&self) -> Pubkey {
        spl_token_2022_interface::id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = StateWithExtensions::<Account>::unpack(data)?;
        Ok(Box::new(Token2022ProgramState(account.base)))
    }

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<solana_sdk::instruction::Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(initialize_account(
            &spl_token_2022_interface::id(),
            account,
            mint,
            owner,
        )?)
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_sdk::instruction::Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(transfer(
            &spl_token_2022_interface::id(),
            source,
            destination,
            authority,
            &[],
            amount,
        )?)
    }

    fn create_transfer_checked_instruction(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_sdk::instruction::Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(transfer_checked(
            &spl_token_2022_interface::id(),
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?)
    }

    fn get_associated_token_address(&self, wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            wallet,
            mint,
            &spl_token_2022_interface::id(),
        )
    }

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        spl_associated_token_account_interface::instruction::create_associated_token_account(
            funding_account,
            wallet,
            mint,
            &spl_token_2022_interface::id(),
        )
    }

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let mint_account = StateWithExtensions::<Mint>::unpack(mint_data)?;
        let transfer_fee_config = mint_account.get_extension::<TransferFeeConfig>().ok().cloned();
        
        Ok(Box::new(Token2022ProgramMint {
            address: *mint,
            mint: mint_account.base,
            transfer_fee_config,
        }))
    }
}
