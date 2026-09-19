use solana_client::client_error::{ClientError, ClientErrorKind};
use std::sync::Arc;

use crate::rpc_failover::FailoverRpcClient;

const HTTP_STATUS_TOO_MANY_REQUESTS: u16 = 429;
const HTTP_STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
const HTTP_STATUS_BAD_GATEWAY: u16 = 502;
const HTTP_STATUS_SERVICE_UNAVAILABLE: u16 = 503;
const HTTP_STATUS_GATEWAY_TIMEOUT: u16 = 504;

impl FailoverRpcClient {
    pub async fn call_with_failover<T, F, Fut>(&self, mut f: F) -> Result<T, ClientError>
    where
        F: FnMut(Arc<solana_client::nonblocking::rpc_client::RpcClient>) -> Fut,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        let endpoint_count = self.endpoint_count();
        let mut last_error: Option<ClientError> = None;

        for attempt in 0..endpoint_count {
            let client = self.get_client();

            match f(client).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if Self::is_retryable_error(&e) {
                        log::warn!(
                            "RPC endpoint {} returned retryable error. Attempting endpoint {}",
                            attempt,
                            (attempt + 1) % endpoint_count
                        );
                        self.rotate_on_failure();
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        log::error!("All RPC endpoints exhausted");
        Err(last_error.unwrap())
    }

    fn is_retryable_error(error: &ClientError) -> bool {
        match error.kind() {
            ClientErrorKind::Reqwest(e) => {
                if let Some(status) = e.status() {
                    let code = status.as_u16();
                    matches!(
                        code,
                        HTTP_STATUS_TOO_MANY_REQUESTS
                            | HTTP_STATUS_INTERNAL_SERVER_ERROR
                            | HTTP_STATUS_BAD_GATEWAY
                            | HTTP_STATUS_SERVICE_UNAVAILABLE
                            | HTTP_STATUS_GATEWAY_TIMEOUT
                    )
                } else {
                    e.is_timeout() || e.is_connect()
                }
            }
            ClientErrorKind::RpcError(_) => {
                let error_str = error.to_string();
                error_str.contains("HTTP status client error (429)")
                    || error_str.contains("HTTP status server error (5")
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_status_constants() {
        assert_eq!(HTTP_STATUS_TOO_MANY_REQUESTS, 429);
        assert_eq!(HTTP_STATUS_INTERNAL_SERVER_ERROR, 500);
        assert_eq!(HTTP_STATUS_BAD_GATEWAY, 502);
        assert_eq!(HTTP_STATUS_SERVICE_UNAVAILABLE, 503);
        assert_eq!(HTTP_STATUS_GATEWAY_TIMEOUT, 504);
    }
}
