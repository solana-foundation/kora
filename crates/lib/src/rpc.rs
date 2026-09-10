use std::{sync::Arc, time::Duration};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;

use crate::rpc_failover::FailoverRpcClient;

/// Create an RPC client from a single endpoint (backward compatibility).
pub fn get_rpc_client(rpc_url: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(90),
        CommitmentConfig::confirmed(),
    ))
}

/// Wrapper that holds either a simple RpcClient or a FailoverRpcClient.
pub enum RpcClientEnum {
    Simple(Arc<RpcClient>),
    Failover(Arc<FailoverRpcClient>),
}

impl RpcClientEnum {
    /// Get the current primary RPC client.
    pub fn get_client(&self) -> Arc<RpcClient> {
        match self {
            RpcClientEnum::Simple(client) => client.clone(),
            RpcClientEnum::Failover(failover) => failover.get_client(),
        }
    }

    /// Get the failover client if it exists (for retry logic).
    pub fn as_failover(&self) -> Option<Arc<FailoverRpcClient>> {
        match self {
            RpcClientEnum::Simple(_) => None,
            RpcClientEnum::Failover(failover) => Some(failover.clone()),
        }
    }
}

/// Create a failover RPC client from multiple endpoints.
///
/// # Arguments
/// * `endpoints` - List of RPC endpoint URLs. If empty, returns an error.
///
/// # Returns
/// An RPC client enum that either wraps a single RpcClient or a FailoverRpcClient.
pub fn get_failover_rpc_client(endpoints: Vec<String>) -> Result<RpcClientEnum, String> {
    if endpoints.is_empty() {
        return Err("At least one RPC endpoint is required".to_string());
    }

    if endpoints.len() == 1 {
        let client = Arc::new(RpcClient::new_with_timeout_and_commitment(
            endpoints[0].clone(),
            Duration::from_secs(90),
            CommitmentConfig::confirmed(),
        ));
        return Ok(RpcClientEnum::Simple(client));
    }

    // Multiple endpoints: return failover client wrapper
    let failover = Arc::new(FailoverRpcClient::new(endpoints));
    Ok(RpcClientEnum::Failover(failover))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rpc_client() {
        let client = get_rpc_client("http://localhost:8899");
        assert!(Arc::strong_count(&client) >= 1);
    }

    #[test]
    fn test_get_failover_rpc_client_single() {
        let result = get_failover_rpc_client(vec!["http://localhost:8899".to_string()]);
        assert!(result.is_ok());
        assert!(result.unwrap().as_failover().is_none());
    }

    #[test]
    fn test_get_failover_rpc_client_multiple() {
        let result = get_failover_rpc_client(vec![
            "http://localhost:8899".to_string(),
            "http://localhost:8900".to_string(),
        ]);
        assert!(result.is_ok());
        assert!(result.unwrap().as_failover().is_some());
    }

    #[test]
    fn test_get_failover_rpc_client_empty() {
        let result = get_failover_rpc_client(vec![]);
        assert!(result.is_err());
    }
}
