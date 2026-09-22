use std::{sync::Arc, time::Duration};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;

use crate::rpc_failover::FailoverRpcClient;

pub fn get_rpc_client(rpc_url: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(90),
        CommitmentConfig::confirmed(),
    ))
}

#[derive(Clone)]
pub enum RpcClientEnum {
    Simple(Arc<RpcClient>),
    Failover(Arc<FailoverRpcClient>),
}

impl RpcClientEnum {
    pub fn get_client(&self) -> Arc<RpcClient> {
        match self {
            RpcClientEnum::Simple(client) => client.clone(),
            RpcClientEnum::Failover(failover) => failover.get_client(),
        }
    }

    pub fn as_failover(&self) -> Option<&FailoverRpcClient> {
        match self {
            RpcClientEnum::Simple(_) => None,
            RpcClientEnum::Failover(failover) => Some(failover.as_ref()),
        }
    }
}

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
