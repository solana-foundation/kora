use crate::oracle::types::{OracleError, OracleProvider};
use async_trait::async_trait;
use lazy_static::lazy_static;
use pyth_sdk_solana::state::SolanaPriceAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::HashMap,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::time::{sleep, Duration};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

lazy_static! {
    static ref PYTH_PRICE_ACCOUNTS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // SOL/USD
        m.insert(
            "So11111111111111111111111111111111111111112",
            "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
        );
        m
    };
}

pub struct PythOracle<'a> {
    rpc_client: &'a RpcClient,
    max_age_secs: u64,
    max_retries: u32,
    retry_delay: Duration,
}

impl<'a> PythOracle<'a> {
    pub fn new(rpc_client: &'a RpcClient) -> Self {
        Self {
            rpc_client,
            max_age_secs: 60,
            max_retries: MAX_RETRIES,
            retry_delay: Duration::from_millis(RETRY_DELAY_MS),
        }
    }

    async fn get_price_with_retry(&self, token_mint: &Pubkey) -> Result<f64, OracleError> {
        let price_key = self.get_price_account(token_mint)?;
        let mut last_error = None;

        for retry in 0..self.max_retries {
            match self.fetch_price(&price_key).await {
                Ok(price) => return Ok(price),
                Err(e) => {
                    if retry < self.max_retries - 1 {
                        log::warn!("Price fetch attempt {} failed: {}, retrying...", retry + 1, e);
                        sleep(self.retry_delay).await;
                        last_error = Some(e);
                    } else {
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or(OracleError::NoPriceAvailable))
    }

    fn get_price_account(&self, token_mint: &Pubkey) -> Result<Pubkey, OracleError> {
        let mint_str = token_mint.to_string();
        let price_account = PYTH_PRICE_ACCOUNTS.get(mint_str.as_str()).ok_or_else(|| {
            OracleError::OracleError(format!("No price feed found for token {}", mint_str))
        })?;

        Pubkey::from_str(price_account)
            .map_err(|e| OracleError::OracleError(format!("Invalid price account pubkey: {}", e)))
    }

    async fn fetch_price(&self, price_key: &Pubkey) -> Result<f64, OracleError> {
        let mut price_account =
            self.rpc_client.get_account(price_key).await.map_err(|e| {
                OracleError::RpcError(format!("Failed to get price account: {}", e))
            })?;

        let price_feed = SolanaPriceAccount::account_to_feed(price_key, &mut price_account)
            .map_err(|e| OracleError::OracleError(format!("Failed to parse price feed: {}", e)))?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| OracleError::OracleError(format!("Time error: {}", e)))?
            .as_secs() as i64;

        let price = price_feed
            .get_price_no_older_than(current_time, self.max_age_secs)
            .ok_or_else(|| OracleError::OracleError("Price unavailable or too old".to_string()))?;

        Ok(price.price as f64 * 10f64.powi(price.expo))
    }
}

#[async_trait]
impl OracleProvider for PythOracle<'_> {
    async fn get_price(&self, token_mint: &Pubkey) -> Result<f64, OracleError> {
        self.get_price_with_retry(token_mint).await
    }
}

#[cfg(test)]
mod tests {
    use crate::constant::SOL_MINT;

    use super::*;
    use mockall::{mock, predicate::*};
    use solana_sdk::commitment_config::CommitmentConfig;

    // Create mock RpcClient for testing
    mock! {
        RpcClient {
            fn get_account_data(&self, pubkey: &Pubkey) -> Result<Vec<u8>, solana_client::client_error::ClientError>;
        }
    }

    #[tokio::test]
    async fn test_get_price_success() {
        let rpc_url = "https://pythnet.rpcpool.com".to_string();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        let oracle = PythOracle::new(&rpc_client);
        let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();

        let price = oracle.get_price(&sol_mint).await;
        assert!(price.is_ok());

        let price_value = price.unwrap();
        assert!(price_value > 0.0);
        println!("SOL price: {}", price_value);
    }

    #[tokio::test]
    async fn test_get_price_invalid_token() {
        let rpc_url = "https://pythnet.rpcpool.com".to_string();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        let oracle = PythOracle::new(&rpc_client);
        let invalid_mint = Pubkey::new_unique();

        let result = oracle.get_price(&invalid_mint).await;
        assert!(matches!(result, Err(OracleError::OracleError(_))));
    }

    #[tokio::test]
    async fn test_price_feed_mapping() {
        let rpc_url = "https://pythnet.rpcpool.com".to_string();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        let oracle = PythOracle::new(&rpc_client);

        // Test all supported tokens
        let tokens = vec![
            "So11111111111111111111111111111111111111112", // SOL
        ];

        for token in tokens {
            let mint = Pubkey::from_str(token).unwrap();
            let price_account = oracle.get_price_account(&mint);
            assert!(price_account.is_ok());
        }
    }
}
