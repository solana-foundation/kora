mod pyth;
mod types;

use std::sync::Arc;

use pyth::PythOracle;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
pub use types::{OracleError, OracleProvider};

use crate::config::ValidationConfig;

#[derive(Clone)]
pub struct OracleClient<'a> {
    pyth: Arc<PythOracle<'a>>,
}

impl<'a> OracleClient<'a> {
    pub async fn new(
        rpc_client: &'a RpcClient,
        config: &ValidationConfig,
    ) -> Result<Self, OracleError> {
        let pyth = Arc::new(PythOracle::create(rpc_client, config).await?);

        Ok(Self { pyth })
    }

    pub async fn get_token_price(&self, token_mint: &Pubkey) -> Result<f64, OracleError> {
        self.pyth.get_price(token_mint).await
    }
}
