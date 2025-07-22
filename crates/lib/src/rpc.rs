use std::{sync::Arc, time::Duration};

use crate::error::KoraError;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

pub fn get_rpc_client(rpc_url: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(90),
        CommitmentConfig::confirmed(),
    ))
}

pub async fn create_rpc_client(url: &str) -> Result<Arc<RpcClient>, KoraError> {
    let client = Arc::new(RpcClient::new(url.to_string()));

    // Test the connection
    client
        .get_version()
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to connect to RPC: {e}")))?;

    Ok(client)
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use std::collections::HashMap;

    pub fn setup_test_rpc_client() -> Arc<RpcClient> {
        let rpc_url = "http://localhost:8899".to_string();
        let mut mocks = HashMap::new();
        mocks.insert(RpcRequest::GetMinimumBalanceForRentExemption, json!(2_039_280));
        Arc::new(RpcClient::new_mock_with_mocks(rpc_url, mocks))
    }
}
