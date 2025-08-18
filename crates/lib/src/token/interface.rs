use async_trait::async_trait;
use mockall::automock;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::any::Any;

pub trait TokenState: Any + Send + Sync {
    fn mint(&self) -> Pubkey;
    fn owner(&self) -> Pubkey;
    fn amount(&self) -> u64;
    fn decimals(&self) -> u8;

    // Add method to support downcasting for Token2022 specific features
    fn as_any(&self) -> &dyn Any;
}

pub trait TokenMint: Any + Send + Sync {
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

    async fn get_and_validate_amount_for_payment<'a>(
        &self,
        rpc_client: &'a RpcClient,
        token_account: Option<&'a dyn TokenState>,
        mint_account: Option<&'a dyn TokenMint>,
        amount: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
}
