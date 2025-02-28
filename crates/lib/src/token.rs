use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::id;
use std::str::FromStr;

use crate::error::KoraError;

mod interface;
mod mint;
mod program;
mod types;

pub use interface::TokenTrait;
pub use mint::{MintStateInterface, TokenMintState};
pub use program::{Token22, TokenKeg};
pub use types::{Token, TokenType};

pub async fn check_valid_token(rpc_client: &RpcClient, token: &str) -> Result<(), KoraError> {
    let pubkey = Pubkey::from_str(token)
        .map_err(|e| KoraError::InternalServerError(format!("Invalid token address: {}", e)))?;

    // Check if the account exists and is a mint account
    match rpc_client.get_account(&pubkey).await {
        Ok(account) => {
            if account.owner == id() {
                Ok(())
            } else {
                Err(KoraError::InternalServerError(format!(
                    "Token {} is not a valid SPL token mint",
                    token
                )))
            }
        }
        Err(e) => {
            Err(KoraError::InternalServerError(format!("Token {} does not exist: {}", token, e)))
        }
    }
}

pub async fn check_valid_tokens(
    rpc_client: &RpcClient,
    tokens: &[String],
) -> Result<(), KoraError> {
    for token in tokens {
        check_valid_token(rpc_client, token).await?;
    }
    Ok(())
}
