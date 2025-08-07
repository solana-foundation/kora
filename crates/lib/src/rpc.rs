use std::{sync::Arc, time::Duration};

use crate::error::KoraError;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;

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
