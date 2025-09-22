#![cfg(test)]

use anyhow::Result;
use jsonrpsee::{
    core::{client::ClientT, traits::ToRpcParams},
    http_client::{HttpClient, HttpClientBuilder},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::common::{
    TransactionBuilder, DEFAULT_RPC_URL, RPC_URL_ENV, TEST_SERVER_URL, TEST_SERVER_URL_ENV,
};

/// Unified test client that manages both HTTP and RPC clients
#[derive(Clone)]
pub struct TestClient {
    pub http_client: HttpClient,
    pub rpc_client: Arc<RpcClient>,
    pub server_url: String,
    pub rpc_url: String,
}

impl TestClient {
    /// Create a new test client with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_urls(Self::get_default_server_url(), Self::get_default_rpc_url()).await
    }

    /// Create a test client with custom URLs
    pub async fn with_urls(server_url: String, rpc_url: String) -> Result<Self> {
        let http_client = HttpClientBuilder::default()
            .request_timeout(std::time::Duration::from_secs(120)) // Increased timeout for CI environments
            .build(&server_url)
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let rpc_client = Arc::new(RpcClient::new(rpc_url.clone()));

        Ok(Self { http_client, rpc_client, server_url, rpc_url })
    }

    /// Make an RPC call to the test server
    pub async fn rpc_call<T, P>(&self, method: &str, params: P) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        P: ToRpcParams + Send,
    {
        self.http_client
            .request(method, params)
            .await
            .map_err(|e| anyhow::anyhow!("RPC call '{}' failed: {}", method, e))
    }

    /// Get the default test server URL (Kora RPC server)
    pub fn get_default_server_url() -> String {
        dotenv::dotenv().ok();
        std::env::var(TEST_SERVER_URL_ENV).unwrap_or_else(|_| TEST_SERVER_URL.to_string())
    }

    /// Get the default RPC URL (Solana RPC)
    pub fn get_default_rpc_url() -> String {
        dotenv::dotenv().ok();
        std::env::var(RPC_URL_ENV).unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
    }
}

/// Test context that provides a unified interface for tests
#[derive(Clone)]
pub struct TestContext {
    pub client: TestClient,
}

impl TestContext {
    /// Create a new test context
    pub async fn new() -> Result<Self> {
        let client = TestClient::new().await?;
        Ok(Self { client })
    }

    /// Create a test context with custom configuration
    pub async fn with_urls(server_url: String, rpc_url: String) -> Result<Self> {
        let client = TestClient::with_urls(server_url, rpc_url).await?;
        Ok(Self { client })
    }

    /// Get the HTTP client for direct JSON-RPC calls
    pub fn http_client(&self) -> &HttpClient {
        &self.client.http_client
    }

    /// Get the Solana RPC client
    pub fn rpc_client(&self) -> &Arc<RpcClient> {
        &self.client.rpc_client
    }

    /// Make an RPC call using the test client
    pub async fn rpc_call<T, P>(&self, method: &str, params: P) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        P: ToRpcParams + Send,
    {
        self.client.rpc_call(method, params).await
    }

    /// Create a transaction builder with the test RPC client
    pub fn transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::legacy().with_rpc_client(self.rpc_client().clone())
    }

    /// Create a V0 transaction builder with the test RPC client  
    pub fn v0_transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::v0().with_rpc_client(self.rpc_client().clone())
    }

    /// Create a V0 transaction builder with lookup tables
    pub fn v0_transaction_builder_with_lookup(
        &self,
        lookup_tables: Vec<solana_sdk::pubkey::Pubkey>,
    ) -> TransactionBuilder {
        TransactionBuilder::v0_with_lookup(lookup_tables).with_rpc_client(self.rpc_client().clone())
    }
}
