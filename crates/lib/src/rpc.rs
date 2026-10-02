use std::{sync::Arc, time::Duration};

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig};
use solana_commitment_config::CommitmentConfig;
use url::Url;

use crate::{constant::RPC_CLIENT_TIMEOUT_SECS, error::KoraError, rpc_failover::FailoverRpcSender};

pub fn get_rpc_client(rpc_url: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(RPC_CLIENT_TIMEOUT_SECS),
        CommitmentConfig::confirmed(),
    ))
}

/// Build the node's RPC client from an ordered list of endpoints.
///
/// A single endpoint keeps the plain HTTP sender, so existing deployments behave
/// exactly as they do today. Two or more are wrapped in a [`FailoverRpcSender`],
/// which retries each RPC call against the next endpoint when the current one is
/// unreachable or answers 5xx. Endpoint order is preference order: the first
/// entry is the primary and is used whenever it is healthy.
///
/// Fails when `endpoints` is empty.
pub fn get_rpc_client_for_endpoints(endpoints: Vec<String>) -> Result<Arc<RpcClient>, KoraError> {
    match endpoints.len() {
        0 => Err(KoraError::ConfigError("At least one RPC endpoint is required".to_string())),
        1 => Ok(get_rpc_client(&endpoints[0])),
        _ => {
            log::info!("{}", describe_endpoints(&endpoints));

            Ok(Arc::new(RpcClient::new_sender(
                FailoverRpcSender::new(endpoints),
                RpcClientConfig::with_commitment(CommitmentConfig::confirmed()),
            )))
        }
    }
}

/// A credential-free summary of the configured endpoints, in preference order.
///
/// The first question during an RPC incident is which endpoint the node is
/// actually talking to, and provider URLs carry the API key in the path or query
/// string, so only hosts are shown.
pub fn describe_endpoints(endpoints: &[String]) -> String {
    let hosts: Vec<String> = endpoints.iter().map(|url| endpoint_host(url)).collect();

    if endpoints.len() > 1 {
        format!("RPC failover enabled across {} endpoints: {}", endpoints.len(), hosts.join(", "))
    } else {
        format!("RPC endpoint: {}", hosts.join(", "))
    }
}

/// The host and port of an endpoint, with no userinfo, path or query.
///
/// RPC endpoint URLs normally carry the provider API key in the path or query
/// string, and occasionally in the userinfo, so only the authority is safe to
/// print. An endpoint that will not parse yields a placeholder rather than any
/// part of the input.
fn endpoint_host(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return "<unparseable endpoint>".to_string();
    };

    let Some(host) = parsed.host_str() else {
        // A URL with no host cannot be dialled, and there is nothing safe to show.
        return "<unparseable endpoint>".to_string();
    };

    // `host_str` already renders an IPv6 literal in brackets, so it can be joined
    // with the port directly.
    match parsed.port_or_known_default() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rpc_client_uses_the_given_url() {
        let client = get_rpc_client("http://localhost:8899");

        assert_eq!(client.url(), "http://localhost:8899");
    }

    #[test]
    fn test_empty_endpoints_are_rejected() {
        let error = match get_rpc_client_for_endpoints(vec![]) {
            Ok(_) => panic!("an empty endpoint list must be rejected"),
            Err(e) => e,
        };

        assert!(error.to_string().contains("At least one RPC endpoint"));
    }

    #[test]
    fn test_single_endpoint_client_reports_that_url() {
        let client = get_rpc_client_for_endpoints(vec!["http://localhost:8899".to_string()])
            .expect("one endpoint is valid");

        assert_eq!(client.url(), "http://localhost:8899");
    }

    #[test]
    fn test_multiple_endpoints_report_the_primary_url() {
        let client = get_rpc_client_for_endpoints(vec![
            "http://primary.example".to_string(),
            "http://secondary.example".to_string(),
        ])
        .expect("two endpoints are valid");

        assert_eq!(client.url(), "http://primary.example");
    }

    #[test]
    fn test_endpoint_host_drops_every_kind_of_credential() {
        for url in [
            "https://mainnet.helius-rpc.com/?api-key=SECRET",
            "https://name.solana-mainnet.quiknode.pro/TOKEN/",
            "https://solana-devnet.g.alchemy.com/v2/SECRET",
        ] {
            assert!(!endpoint_host(url).contains("SECRET"), "{url} leaked a credential");
        }

        assert_eq!(endpoint_host("https://user:SECRET@rpc.example/path"), "rpc.example:443");
        assert_eq!(endpoint_host("http://user:SECRET@rpc.example:8899"), "rpc.example:8899");
        assert_eq!(endpoint_host("https://user@rpc.example"), "rpc.example:443");
    }

    #[test]
    fn test_endpoint_host_shape() {
        assert_eq!(endpoint_host("http://127.0.0.1:8899"), "127.0.0.1:8899");
        assert_eq!(endpoint_host("https://api.devnet.solana.com"), "api.devnet.solana.com:443");
        assert_eq!(
            endpoint_host("http://[::1]:8899"),
            "[::1]:8899",
            "an IPv6 literal needs brackets to stay readable"
        );
    }

    #[test]
    fn test_endpoint_host_refuses_to_echo_an_unparseable_url() {
        for url in ["not a url", "", "://missing-scheme", "http://"] {
            assert_eq!(
                endpoint_host(url),
                "<unparseable endpoint>",
                "{url} must not be echoed into logs"
            );
        }
    }

    #[test]
    fn test_describe_endpoints_never_leaks_a_key() {
        let described = describe_endpoints(&[
            "https://mainnet.helius-rpc.com/?api-key=SECRET".to_string(),
            "https://name.solana-mainnet.quiknode.pro/SECRET/".to_string(),
        ]);

        assert!(
            !described.contains("SECRET"),
            "the summary must not contain an API key: {described}"
        );
        assert!(described.contains("mainnet.helius-rpc.com"));
        assert!(described.contains("name.solana-mainnet.quiknode.pro"));
        assert!(described.contains("2 endpoints"));
    }

    #[test]
    fn test_describe_endpoints_for_a_single_endpoint() {
        let described = describe_endpoints(&["http://127.0.0.1:8899/?api-key=SECRET".to_string()]);

        assert!(
            !described.contains("SECRET"),
            "the summary must not contain an API key: {described}"
        );
        assert!(described.contains("127.0.0.1:8899"));
        assert!(!described.contains("failover"));
    }
}
