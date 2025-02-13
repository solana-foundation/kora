use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OracleError {
    #[error("Price not available")]
    NoPriceAvailable,
    #[error("Oracle error: {0}")]
    OracleError(String),
    #[error("RPC error: {0}")]
    RpcError(String),
}

#[async_trait]
pub trait OracleProvider {
    async fn get_price(&self, token_mint: &Pubkey) -> Result<f64, OracleError>;
}
