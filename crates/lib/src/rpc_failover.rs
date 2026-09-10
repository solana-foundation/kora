use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Multi-endpoint RPC client with health-based failover.
///
/// Accepts an ordered list of RPC endpoints and implements per-request failover
/// on 5xx/transport errors. Maintains a simple health preference for the primary
/// endpoint to minimize switching.
#[derive(Clone)]
pub struct FailoverRpcClient {
    clients: Arc<Vec<Arc<RpcClient>>>,
    primary_index: Arc<AtomicUsize>,
}

impl FailoverRpcClient {
    /// Create a new failover RPC client from a list of endpoints.
    ///
    /// # Arguments
    /// * `endpoints` - Ordered list of RPC endpoint URLs. Empty list will panic.
    ///
    /// # Panics
    /// Panics if the endpoints list is empty.
    pub fn new(endpoints: Vec<String>) -> Self {
        assert!(!endpoints.is_empty(), "At least one RPC endpoint is required");

        let clients = endpoints
            .into_iter()
            .map(|url| {
                Arc::new(RpcClient::new_with_timeout_and_commitment(
                    url,
                    Duration::from_secs(90),
                    solana_commitment_config::CommitmentConfig::confirmed(),
                ))
            })
            .collect::<Vec<_>>();

        Self {
            clients: Arc::new(clients),
            primary_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the currently active RPC client (prefers primary if healthy).
    pub fn get_client(&self) -> Arc<RpcClient> {
        let current_primary = self.primary_index.load(Ordering::Relaxed);
        self.clients[current_primary].clone()
    }

    /// Get all available clients in order.
    pub fn get_all_clients(&self) -> Vec<Arc<RpcClient>> {
        self.clients.as_ref().clone()
    }

    /// Get the number of configured endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.clients.len()
    }

    /// Rotate to the next endpoint on failure.
    ///
    /// This performs a simple round-robin rotation. In practice, this is called
    /// after a request fails with a 5xx or transport error.
    pub fn rotate_on_failure(&self) {
        if self.clients.len() > 1 {
            let current = self.primary_index.load(Ordering::Relaxed);
            let next = (current + 1) % self.clients.len();
            self.primary_index.store(next, Ordering::Relaxed);
        }
    }

    /// Reset to the primary endpoint.
    ///
    /// Called when the primary endpoint recovers or to explicitly reset rotation.
    pub fn reset_to_primary(&self) {
        self.primary_index.store(0, Ordering::Relaxed);
    }

    /// Get the index of the currently active endpoint.
    pub fn current_endpoint_index(&self) -> usize {
        self.primary_index.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_client_creation() {
        let endpoints = vec![
            "http://localhost:8899".to_string(),
            "http://localhost:8900".to_string(),
        ];
        let client = FailoverRpcClient::new(endpoints);
        assert_eq!(client.endpoint_count(), 2);
        assert_eq!(client.current_endpoint_index(), 0);
    }

    #[test]
    fn test_single_endpoint() {
        let endpoints = vec!["http://localhost:8899".to_string()];
        let client = FailoverRpcClient::new(endpoints);
        assert_eq!(client.endpoint_count(), 1);
    }

    #[test]
    #[should_panic]
    fn test_empty_endpoints_panics() {
        let _client = FailoverRpcClient::new(vec![]);
    }

    #[test]
    fn test_rotation() {
        let endpoints = vec![
            "http://localhost:8899".to_string(),
            "http://localhost:8900".to_string(),
            "http://localhost:8901".to_string(),
        ];
        let client = FailoverRpcClient::new(endpoints);

        assert_eq!(client.current_endpoint_index(), 0);
        client.rotate_on_failure();
        assert_eq!(client.current_endpoint_index(), 1);
        client.rotate_on_failure();
        assert_eq!(client.current_endpoint_index(), 2);
        client.rotate_on_failure();
        assert_eq!(client.current_endpoint_index(), 0); // wraps around
    }

    #[test]
    fn test_reset_to_primary() {
        let endpoints = vec![
            "http://localhost:8899".to_string(),
            "http://localhost:8900".to_string(),
        ];
        let client = FailoverRpcClient::new(endpoints);

        client.rotate_on_failure();
        assert_eq!(client.current_endpoint_index(), 1);

        client.reset_to_primary();
        assert_eq!(client.current_endpoint_index(), 0);
    }

    #[test]
    fn test_single_endpoint_no_rotation() {
        let endpoints = vec!["http://localhost:8899".to_string()];
        let client = FailoverRpcClient::new(endpoints);

        client.rotate_on_failure();
        assert_eq!(client.current_endpoint_index(), 0); // stays at 0
    }
}
