use super::interface::{TokenInterface, TokenMint, TokenState};
use async_trait::async_trait;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_interface::{
    instruction::{initialize_account, transfer, transfer_checked},
    state::{Account, Mint},
};
use std::any::Any;

pub struct TokenProgram;

impl TokenProgram {
    pub fn new() -> Self {
        Self
    }
}

pub struct TokenProgramState(Account);

impl TokenState for TokenProgramState {
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

pub struct TokenProgramMint {
    address: Pubkey,
    mint: Mint,
}

impl TokenMint for TokenProgramMint {
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
        Box::new(TokenProgram::new())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        spl_token_interface::id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        use solana_sdk::program_pack::Pack;
        let account = Account::unpack(data)?;
        Ok(Box::new(TokenProgramState(account)))
    }

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(initialize_account(&spl_token_interface::id(), account, mint, owner)?)
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(transfer(
            &spl_token_interface::id(),
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
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(transfer_checked(
            &spl_token_interface::id(),
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
            &spl_token_interface::id(),
        )
    }

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        spl_associated_token_account_interface::instruction::create_associated_token_account(
            funding_account,
            wallet,
            mint,
            &spl_token_interface::id(),
        )
    }

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        use solana_sdk::program_pack::Pack;
        let mint_account = Mint::unpack(mint_data)?;
        Ok(Box::new(TokenProgramMint {
            address: *mint,
            mint: mint_account,
        }))
    }

    async fn get_mint(
        &self,
        rpc_client: &solana_client::nonblocking::rpc_client::RpcClient,
        mint: &Pubkey,
        account_data: Option<Vec<u8>>,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let data = match account_data {
            Some(d) => d,
            None => {
                let account = rpc_client.get_account(mint).await?;
                account.data
            }
        };
        self.unpack_mint(mint, &data)
    }
}
