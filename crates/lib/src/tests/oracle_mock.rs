use crate::{
    error::KoraError,
    oracle::{utils::OracleUtil, PriceOracle, PriceSource, TokenPrice},
};
use reqwest::Client;
use std::{collections::HashMap, sync::Arc, time::Duration};

thread_local! {
    /// Holds a per-test injectable oracle.
    pub static TEST_ORACLE: std::cell::RefCell<Option<Arc<dyn PriceOracle + Send + Sync>>> = std::cell::RefCell::new(None);
}

struct TestOracleProxy;

#[async_trait::async_trait]
impl PriceOracle for TestOracleProxy {
    async fn get_price(
        &self,
        client: &Client,
        mint_address: &str,
    ) -> Result<TokenPrice, KoraError> {
        let oracle = TEST_ORACLE
            .with(|o| o.borrow().clone().unwrap_or_else(|| OracleUtil::get_mock_oracle_price()));
        oracle.get_price(client, mint_address).await
    }

    async fn get_prices(
        &self,
        client: &Client,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        let oracle = TEST_ORACLE.with(|o| {
            if let Some(oracle) = o.borrow().clone() {
                oracle
            } else {
                OracleUtil::get_mock_oracle_price()
            }
        });
        oracle.get_prices(client, mint_addresses).await
    }
}

/// Returns a TestOracleProxy which delegates to TEST_ORACLE.
pub fn get_price_oracle(
    _source: PriceSource,
) -> Result<Arc<dyn PriceOracle + Send + Sync>, KoraError> {
    Ok(Arc::new(TestOracleProxy))
}

/// Simulates a server that never responds.
pub struct HangingOracle {
    pub url: String,
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

/// Binds a TCP listener that accepts connections but never responds.
pub fn spawn_hanging_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_url = format!("http://127.0.0.1:{}", port);

    let _server_handle = std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            std::thread::sleep(Duration::from_secs(30));
            let _ = stream;
        }
    });

    server_url
}
