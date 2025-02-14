use crate::{
    config::ValidationConfig,
    constant::PYTH_PROGRAM_ID,
    oracle::types::{OracleError, OracleProvider},
};
use async_trait::async_trait;
use pyth_sdk_solana::state::{load_product_account, SolanaPriceAccount};
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

pub struct PythOracle<'a> {
    rpc_client: &'a RpcClient,
    max_age_secs: u64,
    max_retries: u32,
    retry_delay: Duration,
    price_mapping: HashMap<String, Pubkey>,
    token_symbols: HashMap<String, String>,
}

impl<'a> PythOracle<'a> {
    pub async fn create(
        rpc_client: &'a RpcClient,
        config: &ValidationConfig,
    ) -> Result<Self, OracleError> {
        let price_mapping = Self::build_price_mapping(rpc_client).await?;

        Ok(Self {
            rpc_client,
            max_age_secs: 60,
            max_retries: MAX_RETRIES,
            retry_delay: Duration::from_millis(RETRY_DELAY_MS),
            price_mapping,
            token_symbols: config.token_symbols.clone(),
        })
    }

    async fn build_price_mapping(
        rpc_client: &RpcClient,
    ) -> Result<HashMap<String, Pubkey>, OracleError> {
        let mut mapping = HashMap::new();

        let pyth_program_id = Pubkey::from_str(PYTH_PROGRAM_ID)
            .map_err(|e| OracleError::OracleError(format!("Invalid Pyth program ID: {}", e)))?;

        let accounts = rpc_client.get_program_accounts(&pyth_program_id).await.map_err(|e| {
            OracleError::OracleError(format!("Failed to get program accounts: {}", e))
        })?;

        for (pubkey, account) in accounts {
            if let Ok(product) = load_product_account(&account.data) {
                // First check if it's a crypto product
                let is_crypto =
                    product.iter().any(|(key, value)| key == "asset_type" && value == "Crypto");

                if is_crypto {
                    let mut base_symbol = None;
                    let mut quote_currency = None;

                    // Then get base and quote symbols
                    for (key, value) in product.iter() {
                        match key {
                            "base" => base_symbol = Some(value),
                            "quote_currency" => quote_currency = Some(value),
                            _ => continue,
                        }
                    }

                    // Only keep USD pairs
                    if let (Some(base), Some(quote)) = (base_symbol, quote_currency) {
                        if quote == "USD" && product.px_acc != Pubkey::default() {
                            mapping.insert(base.to_string(), product.px_acc);
                        }
                    }
                }
            }
        }

        Ok(mapping)
    }

    // Helper function to get token symbol from mint
    fn get_token_symbol(&self, mint: &Pubkey) -> Result<String, OracleError> {
        self.token_symbols
            .get(&mint.to_string())
            .cloned()
            .ok_or_else(|| OracleError::OracleError(format!("Unknown token mint: {}", mint)))
    }

    fn get_price_account(&self, token_mint: &Pubkey) -> Result<Pubkey, OracleError> {
        let symbol = self.get_token_symbol(token_mint)?;

        self.price_mapping.get(&symbol).copied().ok_or_else(|| {
            OracleError::OracleError(format!("No price feed found for token {}", symbol))
        })
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
    use std::fs;

    use crate::{constant::SOL_MINT, load_config, Config};

    use super::*;
    use solana_sdk::commitment_config::CommitmentConfig;
    use tempfile::NamedTempFile;

    fn create_test_config() -> (Config, NamedTempFile) {
        let config_content = r#"
        [validation]
        max_allowed_lamports = 1000000000
        max_signatures = 10
        allowed_programs = ["11111111111111111111111111111111"]
        allowed_tokens = ["So11111111111111111111111111111111111111112", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"]
        allowed_spl_paid_tokens = ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
        disallowed_accounts = []
        token_symbols = {"So11111111111111111111111111111111111111112" = "SOL", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" = "USDC","Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" = "USDT"}

        [kora]
        rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = load_config(temp_file.path()).unwrap_or_else(|e| {
            panic!("Failed to load test config: {}", e);
        });

        (config, temp_file)
    }

    fn create_test_rpc_client() -> RpcClient {
        let rpc_url = "https://pythnet.rpcpool.com".to_string();
        RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed())
    }

    #[tokio::test]
    async fn test_get_price_success() -> Result<(), OracleError> {
        let (config, _temp_file) = create_test_config();
        let rpc_client = create_test_rpc_client();

        let oracle = PythOracle::create(&rpc_client, &config.validation).await?;
        let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();

        let price = oracle.get_price(&sol_mint).await?;
        assert!(price > 0.0);
        println!("SOL price: {}", price);

        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

        let price = oracle.get_price(&usdc_mint).await?;
        assert!(price > 0.0);
        println!("USDC price: {}", price);

        let usdt_mint = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();

        let price = oracle.get_price(&usdt_mint).await?;
        assert!(price > 0.0);
        println!("USDT price: {}", price);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_price_invalid_token() -> Result<(), OracleError> {
        let (config, _temp_file) = create_test_config();
        let rpc_client = create_test_rpc_client();

        let oracle = PythOracle::create(&rpc_client, &config.validation).await?;
        let invalid_mint = Pubkey::new_unique();

        let result = oracle.get_price(&invalid_mint).await;
        assert!(matches!(result, Err(OracleError::OracleError(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_price_feed_mapping() -> Result<(), OracleError> {
        let (config, _temp_file) = create_test_config();
        let rpc_client = create_test_rpc_client();

        let oracle = PythOracle::create(&rpc_client, &config.validation).await?;

        // Test all supported tokens
        let tokens = vec![
            "So11111111111111111111111111111111111111112",  // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
        ];

        for token in tokens {
            let mint = Pubkey::from_str(token).unwrap();
            let price_account = oracle.get_price_account(&mint);
            assert!(price_account.is_ok());
        }
        Ok(())
    }
}
