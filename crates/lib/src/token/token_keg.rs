use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{instruction as token_instruction, state::{Account as TokenAccount, Mint}};

use crate::error::KoraError;

use super::{TokenAccountData, TokenInterface, TokenMintData};

pub struct TokenKeg;

#[async_trait::async_trait]
impl TokenInterface for TokenKeg {
    fn program_id() -> Pubkey {
        spl_token::id()
    }

    async fn create_transfer_instruction(
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, KoraError> {
        token_instruction::transfer_checked(
            &Self::program_id(),
            source,
            &spl_token::id(),
            destination,
            authority,
            &[],
            amount,
            decimals,
        )
        .map_err(|e| KoraError::InvalidTransaction(format!(
            "Failed to create transfer instruction: source={}, dest={}, error={}", 
            source, destination, e
        )))
    }

    fn create_associated_account_instruction(
        payer: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Instruction, KoraError> {
        Ok(create_associated_token_account(
            payer,
            wallet,
            mint,
            &Self::program_id(),
        ))
    }

    fn get_associated_account_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address(wallet, mint)
    }

    async fn get_token_account_data(
        rpc_client: &RpcClient,
        account: &Pubkey,
    ) -> Result<TokenAccountData, KoraError> {
        let account_data = rpc_client
            .get_account(account)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        let token_account = TokenAccount::unpack(&account_data.data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {}", e)))?;

        Ok(TokenAccountData {
            mint: token_account.mint,
            owner: token_account.owner,
            amount: token_account.amount,
        })
    }

    async fn get_mint_data(
        rpc_client: &RpcClient,
        mint: &Pubkey,
    ) -> Result<TokenMintData, KoraError> {
        let mint_account = rpc_client
            .get_account(mint)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        let mint = spl_token::state::Mint::unpack(&mint_account.data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid mint account: {}", e)))?;

        Ok(TokenMintData {
            decimals: mint.decimals,
        })
    }

    fn unpack_transfer_instruction(data: &[u8]) -> Result<u64, KoraError> {
        if let Ok(spl_token::instruction::TokenInstruction::Transfer { amount }) =
            spl_token::instruction::TokenInstruction::unpack(data)
        {
            Ok(amount)
        } else {
            Err(KoraError::InvalidTransaction("Invalid transfer instruction".to_string()))
        }
    }

    fn unpack_account_data(data: &[u8]) -> Result<TokenAccountData, KoraError> {
        let token_account = spl_token::state::Account::unpack(data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {}", e)))?;

        Ok(TokenAccountData {
            mint: token_account.mint,
            owner: token_account.owner,
            amount: token_account.amount,
        })
    }

    fn unpack_mint_data(data: &[u8]) -> Result<TokenMintData, KoraError> {
        let mint = spl_token::state::Mint::unpack(data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid mint account: {}", e)))?;

        Ok(TokenMintData {
            decimals: mint.decimals,
        })
    }
} 