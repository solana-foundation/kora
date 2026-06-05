use crate::{
    error::KoraError,
    oracle::{jupiter::JupiterPriceOracle, utils::OracleUtil},
};
use mockall::automock;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenPrice {
    #[schema(value_type = String)]
    pub price: Decimal,
    pub confidence: f64,
    pub source: PriceSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum PriceSource {
    Jupiter,
    Mock,
}

impl PriceSource {
    /// Stable lowercase identifier used in cache keys; changing these values
    /// invalidates existing Redis entries.
    pub fn as_cache_key(&self) -> &'static str {
        match self {
            PriceSource::Jupiter => "jupiter",
            PriceSource::Mock => "mock",
        }
    }
}

#[automock]
#[async_trait::async_trait]
pub trait PriceOracle {
    async fn get_price(&self, client: &Client, mint_address: &str)
        -> Result<TokenPrice, KoraError>;

    async fn get_prices(
        &self,
        client: &Client,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError>;
}

pub struct RetryingPriceOracle {
    client: Client,
    max_retries: u32,
    base_delay: Duration,
    oracle: Arc<dyn PriceOracle + Send + Sync>,
}

const ORACLE_CONNECT_TIMEOUT_SECS: u64 = 5;
const ORACLE_REQUEST_TIMEOUT_SECS: u64 = 10;

pub fn get_price_oracle(
    source: PriceSource,
) -> Result<Arc<dyn PriceOracle + Send + Sync>, KoraError> {
    match source {
        PriceSource::Jupiter => Ok(Arc::new(JupiterPriceOracle::new()?)),
        PriceSource::Mock => Ok(OracleUtil::get_mock_oracle_price()),
    }
}

impl RetryingPriceOracle {
    pub fn new(
        max_retries: u32,
        base_delay: Duration,
        oracle: Arc<dyn PriceOracle + Send + Sync>,
    ) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(ORACLE_CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(ORACLE_REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to build reqwest client");
        Self { client, max_retries, base_delay, oracle }
    }

    pub async fn get_token_price(&self, mint_address: &str) -> Result<TokenPrice, KoraError> {
        let prices = self.get_token_prices(&[mint_address.to_string()]).await?;

        prices.get(mint_address).cloned().ok_or_else(|| {
            KoraError::InternalServerError("Failed to fetch token price".to_string())
        })
    }

    pub async fn get_token_prices(
        &self,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..self.max_retries {
            let price_result = self.oracle.get_prices(&self.client, mint_addresses).await;

            match price_result {
                Ok(prices) => return Ok(prices),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries - 1 {
                        sleep(delay).await;
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            KoraError::InternalServerError("Failed to fetch token prices".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_price_oracle_retries() {
        let mut mock_oracle = MockPriceOracle::new();
        mock_oracle.expect_get_prices().times(1).returning(|_, mint_addresses| {
            let mut result = HashMap::new();
            for mint in mint_addresses {
                result.insert(
                    mint.clone(),
                    TokenPrice {
                        price: Decimal::from(1),
                        confidence: 0.95,
                        source: PriceSource::Jupiter,
                        block_id: None,
                    },
                );
            }
            Ok(result)
        });

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(100), Arc::new(mock_oracle));
        let result = oracle.get_token_price("test").await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_price_oracle_empty_mints() {
        let mut mock_oracle = MockPriceOracle::new();
        // Expect zero calls — empty mints should short-circuit
        mock_oracle.expect_get_prices().times(0);

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(10), Arc::new(mock_oracle));
        let result = oracle.get_token_prices(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
    #[tokio::test]
    async fn test_price_oracle_retries_all_fail() {
        let mut mock_oracle = MockPriceOracle::new();
        mock_oracle
            .expect_get_prices()
            .times(3)
            .returning(|_, _| Err(KoraError::RpcError("mock error".to_string())));

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(10), Arc::new(mock_oracle));
        let result = oracle.get_token_prices(&["test_mint".to_string()]).await;
        assert!(result.is_err());
        assert_eq!(result.err(), Some(KoraError::RpcError("mock error".to_string())));
    }
    #[tokio::test]
    async fn test_price_oracle_retry_then_succeed() {
        let mut mock_oracle = MockPriceOracle::new();
        let mut call_count = 0u32;
        mock_oracle.expect_get_prices().times(2).returning(move |_, mint_addresses| {
            call_count += 1;
            if call_count == 1 {
                return Err(KoraError::RpcError("temporary error".to_string()));
            }
            let mut result = HashMap::new();
            for mint in mint_addresses {
                result.insert(
                    mint.clone(),
                    TokenPrice {
                        price: Decimal::from(42),
                        confidence: 0.95,
                        source: PriceSource::Jupiter,
                        block_id: None,
                    },
                );
            }
            Ok(result)
        });

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(10), Arc::new(mock_oracle));
        let result = oracle.get_token_price("test_mint").await;
        assert!(result.is_ok());
        let price = result.unwrap();
        assert_eq!(price.price, Decimal::from(42));
    }
    #[tokio::test]
    async fn test_get_token_price_not_found() {
        let mut mock_oracle = MockPriceOracle::new();
        // Return Ok with an empty HashMap — the queried mint won't be in it
        // get_prices returns Ok immediately so retry loop exits after 1 attempt (no retries triggered)
        mock_oracle.expect_get_prices().times(1).returning(|_, _| Ok(HashMap::new()));

        let oracle = RetryingPriceOracle::new(3, Duration::from_millis(10), Arc::new(mock_oracle));
        let result = oracle.get_token_price("missing_mint").await;
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(KoraError::InternalServerError(_))));
    }

    #[tokio::test]
    async fn test_retrying_price_oracle_times_out_on_hanging_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server_url = format!("http://127.0.0.1:{}", port);

        let _server_handle = std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                std::thread::sleep(Duration::from_secs(30));
                let _ = stream;
            }
        });

        struct HangingOracle {
            url: String,
        }

        #[async_trait::async_trait]
        impl PriceOracle for HangingOracle {
            async fn get_price(
                &self,
                _client: &Client,
                _mint_address: &str,
            ) -> Result<TokenPrice, KoraError> {
                unimplemented!()
            }

            async fn get_prices(
                &self,
                client: &Client,
                _mint_addresses: &[String],
            ) -> Result<HashMap<String, TokenPrice>, KoraError> {
                client
                    .get(&self.url)
                    .send()
                    .await
                    .map_err(|e| KoraError::RpcError(format!("Request failed: {}", e)))?;
                Ok(HashMap::new())
            }
        }

        let hanging_oracle = Arc::new(HangingOracle { url: server_url });
        let retrying_oracle =
            RetryingPriceOracle::new(1, Duration::from_millis(10), hanging_oracle);

        let result = tokio::time::timeout(
            Duration::from_secs(15),
            retrying_oracle.get_token_prices(&["dummy_mint".to_string()]),
        )
        .await;

        assert!(result.is_ok(), "Expected request to actually timeout within the window");
        let inner_result = result.unwrap();
        assert!(inner_result.is_err());
    }
}
