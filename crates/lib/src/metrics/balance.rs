use crate::{
    cache::CacheUtil,
    error::KoraError,
    state::{get_config, get_signers_info},
};
use prometheus::{register_gauge_vec, GaugeVec};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, sync::Arc};
use tokio::{
    sync::OnceCell,
    task::JoinHandle,
    time::{interval, Duration},
};

/// Global Prometheus gauge vector for tracking all signer balances
static SIGNER_BALANCE_GAUGES: OnceCell<GaugeVec> = OnceCell::const_new();

/// Balance tracker for monitoring signer SOL balance
pub struct BalanceTracker;

impl BalanceTracker {
    /// Initialize the Prometheus gauge vector for multi-signer balance tracking
    pub async fn init() -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        let gauge_vec = register_gauge_vec!(
            "signer_balance_lamports",
            "Current SOL balance of each signer in lamports",
            &["signer_name", "signer_pubkey"]
        )
        .map_err(|e| {
            KoraError::InternalServerError(format!("Failed to register balance gauge vector: {e}"))
        })?;

        SIGNER_BALANCE_GAUGES.set(gauge_vec).map_err(|_| {
            KoraError::InternalServerError("Balance gauge vector already initialized".to_string())
        })?;

        log::info!("Multi-signer balance tracking metrics initialized");
        Ok(())
    }

    /// Track all signers' balances and update Prometheus metrics
    pub async fn track_all_signer_balances(rpc_client: &Arc<RpcClient>) -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        // Get all signers in the pool
        let signers_info = get_signers_info()?;

        if let Some(gauge_vec) = SIGNER_BALANCE_GAUGES.get() {
            let mut balance_results = Vec::new();

            // Batch fetch all signer balances
            for signer_info in &signers_info {
                let pubkey = Pubkey::from_str(&signer_info.public_key).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid signer pubkey {}: {e}",
                        signer_info.public_key
                    ))
                })?;

                match CacheUtil::get_account(rpc_client, &pubkey, false).await {
                    Ok(account) => {
                        balance_results.push((signer_info, account.lamports));
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to get balance for signer {} ({}): {e}",
                            signer_info.name,
                            signer_info.public_key
                        );
                        // Set balance to 0 on error to indicate issue
                        balance_results.push((signer_info, 0));
                    }
                }
            }

            // Update all gauge metrics
            for (signer_info, balance_lamports) in balance_results {
                let gauge =
                    gauge_vec.with_label_values(&[&signer_info.name, &signer_info.public_key]);

                gauge.set(balance_lamports as f64);

                log::debug!(
                    "Updated balance metrics: {} lamports for signer {} ({})",
                    balance_lamports,
                    signer_info.name,
                    signer_info.public_key
                );
            }
        } else {
            log::warn!("Balance gauge vector not initialized, skipping metrics update");
        }

        Ok(())
    }

    /// Start a background task that tracks balance at regular intervals
    /// Returns a JoinHandle to allow for proper task shutdown
    pub async fn start_background_tracking(rpc_client: Arc<RpcClient>) -> Option<JoinHandle<()>> {
        if !BalanceTracker::is_enabled() {
            log::info!("Balance tracking is disabled, not starting background task");
            return None;
        }

        let config = match get_config() {
            Ok(config) => config,
            Err(e) => {
                log::error!("Failed to get config for balance tracking: {e}");
                return None;
            }
        };

        let interval_seconds = config.metrics.fee_payer_balance.expiry_seconds;
        log::info!("Starting multi-signer balance tracking background task with {interval_seconds}s interval");

        // Spawn a background task that runs forever
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                // Track all signer balances, but don't let errors crash the loop
                if let Err(e) = BalanceTracker::track_all_signer_balances(&rpc_client).await {
                    log::warn!("Failed to track signer balances in background task: {e}");
                }
            }
        });

        Some(handle)
    }

    pub fn is_enabled() -> bool {
        match get_config() {
            Ok(config) => config.metrics.enabled && config.metrics.fee_payer_balance.enabled,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            Config, FeePayerBalanceMetricsConfig, KoraConfig, MetricsConfig, ValidationConfig,
        },
        fee::price::PriceConfig,
        oracle::PriceSource,
        signer::{
            memory_signer::solana_signer::SolanaMemorySigner, KoraSigner, SignerPool,
            SignerWithMetadata,
        },
        state::{update_config, update_signer_pool},
    };
    use serial_test::serial;
    use solana_sdk::signature::Keypair;

    fn create_test_config(balance_metrics_enabled: bool) -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1000000,
                max_signatures: 10,
                allowed_programs: vec![],
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock,
                fee_payer_policy: crate::config::FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: crate::config::Token2022Config::default(),
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: crate::config::EnabledMethods::default(),
                auth: crate::config::AuthConfig::default(),
                payment_address: None,
                cache: crate::config::CacheConfig {
                    url: None,
                    enabled: false,
                    default_ttl: 300,
                    account_ttl: 60,
                },
            },
            metrics: MetricsConfig {
                enabled: true,
                endpoint: "/metrics".to_string(),
                port: 8080,
                scrape_interval: 60,
                fee_payer_balance: FeePayerBalanceMetricsConfig {
                    enabled: balance_metrics_enabled,
                    expiry_seconds: 30,
                },
            },
        }
    }

    fn create_test_signer_pool() -> SignerPool {
        let signer1 = SolanaMemorySigner::new(Keypair::new());
        let signer2 = SolanaMemorySigner::new(Keypair::new());

        SignerPool::new(vec![
            SignerWithMetadata::new("test_signer_1".to_string(), KoraSigner::Memory(signer1), 1),
            SignerWithMetadata::new("test_signer_2".to_string(), KoraSigner::Memory(signer2), 1),
        ])
    }

    #[tokio::test]
    #[serial]
    async fn test_balance_tracking_disabled() {
        let config = create_test_config(false);
        let _ = update_config(config);

        // Balance tracking should report as disabled
        assert!(!BalanceTracker::is_enabled());
    }

    #[tokio::test]
    #[serial]
    async fn test_balance_tracking_enabled() {
        let config = create_test_config(true);
        let _ = update_config(config);

        // Balance tracking should report as enabled
        assert!(BalanceTracker::is_enabled());
    }

    #[tokio::test]
    #[serial]
    async fn test_multi_signer_balance_tracking() {
        let config = create_test_config(true);
        let _ = update_config(config);

        // Set up test signer pool
        let pool = create_test_signer_pool();
        let _ = update_signer_pool(pool);

        // Verify we can get signers info
        let signers_info = get_signers_info().expect("Should get signers info");
        assert_eq!(signers_info.len(), 2);
        assert_eq!(signers_info[0].name, "test_signer_1");
        assert_eq!(signers_info[1].name, "test_signer_2");

        // Verify public keys are valid Solana pubkeys
        for signer_info in signers_info {
            assert!(Pubkey::from_str(&signer_info.public_key).is_ok());
        }
    }
}
