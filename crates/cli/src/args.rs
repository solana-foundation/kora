use std::collections::HashSet;

use clap::Parser;

/// Global arguments used by all subcommands
#[derive(Debug, Parser)]
#[command(name = "kora")]
pub struct GlobalArgs {
    /// Solana RPC endpoint URLs, comma separated, tried in order.
    ///
    /// With more than one endpoint each RPC call is retried against the next
    /// endpoint when the current one is unreachable or answers 5xx. The first
    /// entry is the primary and is used whenever it is healthy. Takes precedence
    /// over --rpc-url.
    #[arg(long, env = "RPC_URLS", value_delimiter = ',')]
    pub rpc_urls: Vec<String>,

    /// Solana RPC endpoint URL. Ignored when RPC_URLS or --rpc-urls is set.
    #[arg(long, env = "RPC_URL", default_value = "http://127.0.0.1:8899")]
    pub rpc_url: String,

    /// Path to Kora configuration file (TOML format)
    #[arg(long, default_value = "kora.toml")]
    pub config: String,
}

impl GlobalArgs {
    /// The RPC endpoints to use, in preference order.
    ///
    /// `RPC_URLS`/`--rpc-urls` wins when it holds at least one non-empty entry, so
    /// an operator who sets `RPC_URLS=` to an empty value falls back to
    /// `RPC_URL` instead of failing to start. Blank entries are dropped and
    /// duplicates are collapsed, so a repeated URL cannot quietly consume part of
    /// the failover attempt budget.
    pub fn rpc_endpoints(&self) -> Result<Vec<String>, String> {
        let mut endpoints: Vec<String> = self
            .rpc_urls
            .iter()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .collect();

        if endpoints.is_empty() {
            let single = self.rpc_url.trim().to_string();
            if single.is_empty() {
                return Err(
                    "At least one RPC endpoint is required: set RPC_URLS or RPC_URL".to_string()
                );
            }

            endpoints.push(single);
        }

        let mut seen = HashSet::with_capacity(endpoints.len());
        endpoints.retain(|url| seen.insert(url.clone()));

        Ok(endpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(rpc_urls: Vec<&str>, rpc_url: &str) -> GlobalArgs {
        GlobalArgs {
            rpc_urls: rpc_urls.into_iter().map(str::to_string).collect(),
            rpc_url: rpc_url.to_string(),
            config: "kora.toml".to_string(),
        }
    }

    #[test]
    fn test_rpc_urls_are_used_in_order() {
        let endpoints =
            args(vec!["http://primary:8899", "http://secondary:8899"], "http://ignored:8899")
                .rpc_endpoints()
                .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899", "http://secondary:8899"]);
    }

    #[test]
    fn test_rpc_urls_take_precedence_over_rpc_url() {
        let endpoints = args(vec!["http://primary:8899"], "http://ignored:8899")
            .rpc_endpoints()
            .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899"]);
    }

    #[test]
    fn test_falls_back_to_rpc_url_when_rpc_urls_is_absent() {
        let endpoints =
            args(vec![], "http://fallback:8899").rpc_endpoints().expect("should resolve");

        assert_eq!(endpoints, vec!["http://fallback:8899"]);
    }

    #[test]
    fn test_empty_rpc_urls_falls_back_to_rpc_url() {
        let endpoints =
            args(vec![""], "http://fallback:8899").rpc_endpoints().expect("should resolve");

        assert_eq!(endpoints, vec!["http://fallback:8899"]);
    }

    #[test]
    fn test_whitespace_only_rpc_urls_fall_back_to_rpc_url() {
        let endpoints =
            args(vec!["   "], "http://fallback:8899").rpc_endpoints().expect("resolves");

        assert_eq!(endpoints, vec!["http://fallback:8899"]);
    }

    #[test]
    fn test_blank_entries_are_dropped() {
        let endpoints =
            args(vec!["http://primary:8899", "", "  ", "http://secondary:8899"], "http://ignored")
                .rpc_endpoints()
                .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899", "http://secondary:8899"]);
    }

    #[test]
    fn test_entries_are_trimmed() {
        let endpoints = args(vec!["  http://primary:8899  ", "http://secondary:8899"], "http://x")
            .rpc_endpoints()
            .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899", "http://secondary:8899"]);
    }

    #[test]
    fn test_duplicate_endpoints_are_collapsed() {
        let endpoints = args(
            vec!["http://primary:8899", "http://secondary:8899", "http://primary:8899"],
            "http://ignored",
        )
        .rpc_endpoints()
        .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899", "http://secondary:8899"]);
    }

    #[test]
    fn test_single_endpoint_is_not_collapsed() {
        let endpoints = args(vec!["http://primary:8899"], "http://ignored")
            .rpc_endpoints()
            .expect("should resolve");

        assert_eq!(endpoints, vec!["http://primary:8899"]);
    }

    #[test]
    fn test_no_endpoints_at_all_is_an_error() {
        let error = args(vec![], "").rpc_endpoints().expect_err("nothing configured");

        assert!(error.contains("RPC_URLS"));
    }
}
