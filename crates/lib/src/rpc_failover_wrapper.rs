use solana_client::client_error::ClientError;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::rpc_failover::FailoverRpcClient;

/// Wraps RPC calls with automatic failover on 5xx/transport errors.
///
/// When a primary RPC endpoint returns 5xx or transport errors, automatically
/// retries on the next endpoint in the failover chain.
pub struct FailoverRpcCallWrapper {
    failover_client: Arc<FailoverRpcClient>,
}

impl FailoverRpcCallWrapper {
    /// Create a new failover wrapper from a FailoverRpcClient.
    pub fn new(failover_client: Arc<FailoverRpcClient>) -> Self {
        Self { failover_client }
    }

    /// Execute an RPC call with automatic failover on 5xx/transport errors.
    ///
    /// # Arguments
    /// * `f` - A closure that takes an RpcClient and returns a Result
    ///
    /// # Returns
    /// The result of the RPC call, or an error if all endpoints fail.
    pub async fn call_with_failover<T, F, Fut>(&self, mut f: F) -> Result<T, ClientError>
    where
        F: FnMut(Arc<RpcClient>) -> Fut,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        let endpoint_count = self.failover_client.endpoint_count();
        let mut last_error: Option<ClientError> = None;

        // Try each endpoint in order
        for attempt in 0..endpoint_count {
            let client = self.failover_client.get_client();

            match f(client).await {
                Ok(result) => {
                    // Success - return immediately
                    return Ok(result);
                }
                Err(e) => {
                    // Check if this is a 5xx or transport error
                    if Self::is_retryable_error(&e) {
                        log::warn!(
                            "RPC endpoint {} returned retryable error. Attempting endpoint {}",
                            attempt,
                            (attempt + 1) % endpoint_count
                        );
                        self.failover_client.rotate_on_failure();
                        last_error = Some(e);
                        // Continue to next endpoint
                    } else {
                        // Non-retryable error - return immediately
                        return Err(e);
                    }
                }
            }
        }

        // All endpoints exhausted
        log::error!("All RPC endpoints exhausted");
        Err(last_error.unwrap())
    }

    /// Check if an error is retryable (5xx or transport error).
    fn is_retryable_error(error: &ClientError) -> bool {
        let error_str = error.to_string();

        // Check for 5xx HTTP status codes in error message
        if error_str.contains("500") || error_str.contains("502") || error_str.contains("503")
            || error_str.contains("504")
        {
            return true;
        }

        // Check for rate limit errors
        if error_str.contains("429") || error_str.contains("rate limit") {
            return true;
        }

        // Check for transport/connection errors
        if error_str.contains("connection") || error_str.contains("timeout")
            || error_str.contains("ConnectError")
            || error_str.contains("Timeout")
        {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_is_retryable_error_500() {
        let error_str = "500 Internal Server Error";
        assert!(error_str.contains("500"));
    }

    #[test]
    fn test_is_retryable_error_503() {
        let error_str = "503 Service Unavailable";
        assert!(error_str.contains("503"));
    }

    #[test]
    fn test_is_retryable_error_connection() {
        let error_str = "connection refused";
        assert!(error_str.contains("connection"));
    }

    #[test]
    fn test_is_retryable_error_timeout() {
        let error_str = "request timeout";
        assert!(error_str.contains("timeout"));
    }

    #[test]
    fn test_non_retryable_error() {
        let error_str = "invalid account";
        assert!(!error_str.contains("500") && !error_str.contains("connection"));
    }
}
