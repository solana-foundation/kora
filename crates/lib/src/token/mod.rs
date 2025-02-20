use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::error::KoraError;

pub mod token_keg;

/// Interface for token program interactions
#[async_trait::async_trait]
pub trait TokenInterface {
    /// Returns the program ID for the token implementation
    /// 
    /// This is used to identify which program handles the token instructions
    fn program_id() -> Pubkey;

    /// Creates a transfer instruction for moving tokens between accounts
    /// 
    /// # Arguments
    /// * `source` - Source token account
    /// * `destination` - Destination token account
    /// * `authority` - Account authorized to transfer tokens
    /// * `amount` - Amount of tokens to transfer
    /// * `decimals` - Decimals of the token mint
    async fn create_transfer_instruction(
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, KoraError>;

    /// Create an instruction to create an associated token account
    fn create_associated_account_instruction(
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, KoraError>;

    /// Get the associated token account address
    fn get_associated_account_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey;

    /// Get token account data including amount and mint
    async fn get_token_account_data(
        rpc_client: &RpcClient,
        account: &Pubkey,
    ) -> Result<TokenAccountData, KoraError>;

    /// Get mint data including decimals
    async fn get_mint_data(
        rpc_client: &RpcClient,
        mint: &Pubkey,
    ) -> Result<TokenMintData, KoraError>;

    /// Unpack a transfer instruction to get the amount
    fn unpack_transfer_instruction(data: &[u8]) -> Result<u64, KoraError>;

    /// Unpack account data into TokenAccountData
    fn unpack_account_data(data: &[u8]) -> Result<TokenAccountData, KoraError>;

    /// Unpack mint data into TokenMintData
    fn unpack_mint_data(data: &[u8]) -> Result<TokenMintData, KoraError>;
}

#[derive(Debug, Clone)]
pub struct TokenAccountData {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct TokenMintData {
    pub decimals: u8,
} 