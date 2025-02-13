mod pyth;
mod types;

use pyth::PythOracle;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
pub use types::{OracleError, OracleProvider};

pub struct OracleClient<'a> {
    pyth: PythOracle<'a>,
}

impl<'a> OracleClient<'a> {
    pub fn new(rpc_client: &'a RpcClient) -> Self {
        Self { pyth: PythOracle::new(rpc_client) }
    }

    pub async fn get_token_price(&self, token_mint: &Pubkey) -> Result<f64, OracleError> {
        self.pyth.get_price(token_mint).await
    }
}
