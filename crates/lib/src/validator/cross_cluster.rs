use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use std::{str::FromStr, time::Duration};
use tokio::time::timeout;

/// Time in seconds before giving up on a cross-cluster probe to avoid hanging validation.
const CROSS_CLUSTER_TIMEOUT_SECS: u64 = 5;

pub(crate) enum ProbeOutcome {
    Found(Vec<String>),
    NotFound,
    /// RPC error or timeout cannot conclude the mint is absent on this cluster.
    Failed,
}

pub(crate) async fn check_cross_cluster_mints(
    rpc_client: &RpcClient,
    tokens: &[String],
    endpoints: &[String],
    warnings: &mut Vec<String>,
) {
    let pubkeys: Vec<(String, Pubkey)> =
        tokens.iter().filter_map(|t| Pubkey::from_str(t).ok().map(|pk| (t.clone(), pk))).collect();

    if pubkeys.is_empty() {
        return;
    }

    let pks: Vec<Pubkey> = pubkeys.iter().map(|(_, pk)| *pk).collect();
    let accounts = match rpc_client.get_multiple_accounts(&pks).await {
        Ok(a) => a,
        Err(_) => {
            warnings.push(
                "cross-cluster check skipped (could not reach connected cluster)".to_string(),
            );
            return;
        }
    };

    let missing: Vec<String> = pubkeys
        .iter()
        .zip(accounts.iter())
        .filter_map(|((addr, _), acct)| acct.is_none().then_some(addr.clone()))
        .collect();

    probe_missing_mints(&missing, endpoints, warnings).await;
}

pub(crate) async fn probe_missing_mints(
    missing: &[String],
    endpoints: &[String],
    warnings: &mut Vec<String>,
) {
    if missing.is_empty() {
        return;
    }

    if endpoints.is_empty() {
        warnings.push("cross-cluster check enabled but no endpoints configured".to_string());
        return;
    }

    let probe_futures: Vec<_> = endpoints
        .iter()
        .map(|rpc_url| {
            let missing = missing.to_vec();
            let url = rpc_url.clone();
            async move {
                let client = RpcClient::new(url.clone());
                let missing_pks: Vec<Pubkey> =
                    missing.iter().filter_map(|addr| Pubkey::from_str(addr).ok()).collect();

                let result = timeout(
                    Duration::from_secs(CROSS_CLUSTER_TIMEOUT_SECS),
                    client.get_multiple_accounts(&missing_pks),
                )
                .await;

                match result {
                    Ok(Ok(accounts)) => {
                        let found: Vec<String> = missing
                            .iter()
                            .zip(accounts.iter())
                            .filter_map(|(addr, acct)| acct.is_some().then_some(addr.clone()))
                            .collect();
                        if found.is_empty() {
                            (url, ProbeOutcome::NotFound)
                        } else {
                            (url, ProbeOutcome::Found(found))
                        }
                    }
                    Ok(Err(_)) => (url, ProbeOutcome::Failed),
                    Err(_) => (url, ProbeOutcome::Failed),
                }
            }
        })
        .collect();

    let probe_results = futures::future::join_all(probe_futures).await;
    emit_cluster_warnings(missing, &probe_results, warnings);
}

pub(crate) fn emit_cluster_warnings(
    missing: &[String],
    probe_results: &[(String, ProbeOutcome)],
    warnings: &mut Vec<String>,
) {
    for mint_addr in missing {
        let found_on: Vec<&str> = probe_results
            .iter()
            .filter_map(|(cluster_name, outcome)| match outcome {
                ProbeOutcome::Found(mints) if mints.contains(mint_addr) => {
                    Some(cluster_name.as_str())
                }
                _ => None,
            })
            .collect();

        let conclusive: Vec<&str> = probe_results
            .iter()
            .filter_map(|(name, outcome)| match outcome {
                ProbeOutcome::Found(_) | ProbeOutcome::NotFound => Some(name.as_str()),
                ProbeOutcome::Failed => None,
            })
            .collect();

        let warning = if !found_on.is_empty() {
            format!(
                "mint {} not found on the connected cluster\n  found on: {}\n  possible cluster mismatch",
                mint_addr,
                found_on.join(", ")
            )
        } else if conclusive.is_empty() {
            format!(
                "mint {} not found on the connected cluster\n  cross-cluster check inconclusive (all probes failed or timed out)",
                mint_addr,
            )
        } else {
            format!(
                "mint {} not found on the connected cluster or on: {}",
                mint_addr,
                conclusive.join(", ")
            )
        };

        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice::from_ref;

    #[test]
    fn test_missing_mint_triggers_warning() {
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string();
        let probe_results: Vec<(String, ProbeOutcome)> = vec![
            ("devnet".into(), ProbeOutcome::NotFound),
            ("testnet".into(), ProbeOutcome::Failed),
        ];
        let mut warnings = Vec::new();
        emit_cluster_warnings(from_ref(&mint), &probe_results, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains(&mint)
                && warnings[0].contains("not found on the connected cluster or on:"),
            "unexpected warning: {}",
            warnings[0]
        );
    }

    #[test]
    fn test_all_probes_failed() {
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string();
        let probe_results: Vec<(String, ProbeOutcome)> =
            vec![("devnet".into(), ProbeOutcome::Failed), ("testnet".into(), ProbeOutcome::Failed)];
        let mut warnings = Vec::new();
        emit_cluster_warnings(from_ref(&mint), &probe_results, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains(&mint) && warnings[0].contains("cross-cluster check inconclusive"),
            "unexpected warning: {}",
            warnings[0]
        );
    }

    #[test]
    fn test_existing_mint_no_warning() {
        let probe_results: Vec<(String, ProbeOutcome)> = vec![];
        let mut warnings = Vec::new();
        emit_cluster_warnings(&[], &probe_results, &mut warnings);

        assert!(warnings.is_empty());
    }

    #[test]
    fn test_mint_found_on_other_cluster_warning() {
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string();
        let probe_results: Vec<(String, ProbeOutcome)> = vec![
            ("devnet".into(), ProbeOutcome::Found(vec![mint.clone()])),
            ("testnet".into(), ProbeOutcome::NotFound),
        ];
        let mut warnings = Vec::new();
        emit_cluster_warnings(from_ref(&mint), &probe_results, &mut warnings);

        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains(&mint), "mint address missing from warning");
        assert!(w.contains("found on:"), "expected 'found on:' in warning");
        assert!(w.contains("devnet"), "expected cluster name in warning");
        assert!(w.contains("possible cluster mismatch"), "expected mismatch note");
    }
}
