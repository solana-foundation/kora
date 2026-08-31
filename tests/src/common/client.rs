#![cfg(test)]

use anyhow::Result;
use jsonrpsee::{
    core::{client::ClientT, traits::ToRpcParams},
    http_client::{HttpClient, HttpClientBuilder},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
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
    pub async fn new() -> Result<Self> {
        Self::with_urls(Self::get_default_server_url(), Self::get_default_rpc_url()).await
    }

    pub async fn with_urls(server_url: String, rpc_url: String) -> Result<Self> {
        let http_client = HttpClientBuilder::default()
            .build(&server_url)
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));

        Ok(Self { http_client, rpc_client, server_url, rpc_url })
    }

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

    pub fn get_default_server_url() -> String {
        dotenv::dotenv().ok();
        std::env::var(TEST_SERVER_URL_ENV).unwrap_or_else(|_| TEST_SERVER_URL.to_string())
    }

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
    pub async fn new() -> Result<Self> {
        let client = TestClient::new().await?;
        Ok(Self { client })
    }

    pub async fn with_urls(server_url: String, rpc_url: String) -> Result<Self> {
        let client = TestClient::with_urls(server_url, rpc_url).await?;
        Ok(Self { client })
    }

    pub fn http_client(&self) -> &HttpClient {
        &self.client.http_client
    }

    pub fn rpc_client(&self) -> &Arc<RpcClient> {
        &self.client.rpc_client
    }

    pub async fn rpc_call<T, P>(&self, method: &str, params: P) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        P: ToRpcParams + Send,
    {
        self.client.rpc_call(method, params).await
    }

    pub fn transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::legacy().with_rpc_client(self.rpc_client().clone())
    }

    pub fn v0_transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::v0().with_rpc_client(self.rpc_client().clone())
    }

    pub fn v0_transaction_builder_with_lookup(
        &self,
        lookup_tables: Vec<solana_sdk::pubkey::Pubkey>,
    ) -> TransactionBuilder {
        TransactionBuilder::v0_with_lookup(lookup_tables).with_rpc_client(self.rpc_client().clone())
    }

    pub fn v1_transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::v1().with_rpc_client(self.rpc_client().clone())
    }
}
