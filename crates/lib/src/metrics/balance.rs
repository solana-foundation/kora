#[cfg(not(test))]
use crate::state::get_config;
#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;
use crate::{error::KoraError, sanitize_error, signer::SignerInfo, state::get_signers_info};
use prometheus::{register_counter_vec, register_gauge_vec, CounterVec, GaugeVec};
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

static SIGNER_BALANCE_FETCH_ERRORS: OnceCell<CounterVec> = OnceCell::const_new();

/// Balance tracker for monitoring signer SOL balance
pub struct BalanceTracker;

impl BalanceTracker {
    /// Initialize the Prometheus gauge vector for multi-signer balance tracking
    pub async fn init() -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        // Register exactly once per process. The Prometheus default registry and the OnceCell are
        // process-global, so a plain register + set would error on any second call (e.g. across
        // tests); get_or_try_init keeps init idempotent and safe under concurrent callers.
        SIGNER_BALANCE_GAUGES
            .get_or_try_init(|| async {
                let gauge_vec = register_gauge_vec!(
                    "signer_balance_lamports",
                    "Current SOL balance of each signer in lamports",
                    &["signer_name", "signer_pubkey"]
                )
                .map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Failed to register balance gauge vector: {e}"
                    ))
                })?;
                log::info!("Multi-signer balance tracking metrics initialized");
                Ok::<_, KoraError>(gauge_vec)
            })
            .await?;

        SIGNER_BALANCE_FETCH_ERRORS
            .get_or_try_init(|| async {
                register_counter_vec!(
                    "signer_balance_fetch_errors_total",
                    "Failed signer balance fetches; signer_balance_lamports keeps its last value",
                    &["signer_name", "signer_pubkey"]
                )
                .map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Failed to register balance fetch error counter: {e}"
                    ))
                })
            })
            .await?;

        Ok(())
    }

    /// Track all signers' balances and update Prometheus metrics
    pub async fn track_all_signer_balances(rpc_client: &Arc<RpcClient>) -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        let signers_info = get_signers_info()?;

        let (Some(gauge_vec), Some(fetch_errors)) =
            (SIGNER_BALANCE_GAUGES.get(), SIGNER_BALANCE_FETCH_ERRORS.get())
        else {
            log::warn!("Balance metrics not initialized, skipping metrics update");
            return Ok(());
        };

        for signer_info in &signers_info {
            let pubkey = Pubkey::from_str(&signer_info.public_key).map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Invalid signer pubkey {}: {e}",
                    signer_info.public_key
                ))
            })?;

            Self::update_signer_balance(rpc_client, gauge_vec, fetch_errors, signer_info, &pubkey)
                .await;
        }

        Ok(())
    }

    /// `getBalance` returns 0 for a missing account, so an error always means the fetch failed.
    async fn update_signer_balance(
        rpc_client: &RpcClient,
        gauge_vec: &GaugeVec,
        fetch_errors: &CounterVec,
        signer_info: &SignerInfo,
        pubkey: &Pubkey,
    ) {
        let labels = [signer_info.name.as_str(), signer_info.public_key.as_str()];

        match rpc_client.get_balance(pubkey).await {
            Ok(balance_lamports) => {
                gauge_vec.with_label_values(&labels).set(balance_lamports as f64);

                log::debug!(
                    "Updated balance metrics: {} lamports for signer {} ({})",
                    balance_lamports,
                    signer_info.name,
                    signer_info.public_key
                );
            }
            Err(e) => {
                fetch_errors.with_label_values(&labels).inc();

                log::warn!(
                    "Failed to fetch balance for signer {} ({}): {}",
                    signer_info.name,
                    signer_info.public_key,
                    sanitize_error!(e)
                );
            }
        }
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

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

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
        config::FeePayerBalanceMetricsConfig,
        signer::{pool::SignerWithMetadata, SignerPool},
        state::update_signer_pool,
        tests::{
            common::RpcMockBuilder,
            config_mock::{ConfigMockBuilder, MetricsConfigBuilder},
        },
    };
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use solana_keychain::Signer;
    use solana_sdk::signature::Keypair;

    fn setup_test_signer_pool() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();

        let external_signer1 = Signer::from_memory(&keypair1.to_base58_string()).unwrap();
        let external_signer2 = Signer::from_memory(&keypair2.to_base58_string()).unwrap();

        let pool = SignerPool::new(vec![
            SignerWithMetadata::new("signer_1".to_string(), Arc::new(external_signer1), 1),
            SignerWithMetadata::new("signer_2".to_string(), Arc::new(external_signer2), 2),
        ]);

        let _ = update_signer_pool(pool);
    }

    #[tokio::test]
    async fn test_is_enabled_when_disabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(false)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: false,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        assert!(!BalanceTracker::is_enabled());
    }

    #[tokio::test]
    async fn test_is_enabled_when_enabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(true)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: true,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        assert!(BalanceTracker::is_enabled());
    }

    #[tokio::test]
    async fn test_is_enabled_requires_both_flags() {
        // Test case: metrics enabled but balance metrics disabled
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(true)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: false,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        assert!(!BalanceTracker::is_enabled());
    }

    #[tokio::test]
    async fn test_is_enabled_metrics_disabled_balance_enabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(false)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: true,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        assert!(!BalanceTracker::is_enabled());
    }

    #[tokio::test]
    async fn test_init_when_disabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(false)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: false,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        let result = BalanceTracker::init().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_when_enabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(true)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: true,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        let result = BalanceTracker::init().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_track_all_signer_balances_when_disabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(false)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: false,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        let mock_rpc = RpcMockBuilder::new().build();
        let result = BalanceTracker::track_all_signer_balances(&mock_rpc).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_background_tracking_when_disabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(false)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: false,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        let mock_rpc = RpcMockBuilder::new().build();
        let handle = BalanceTracker::start_background_tracking(mock_rpc).await;

        assert!(handle.is_none());
    }

    #[tokio::test]
    async fn test_start_background_tracking_when_enabled() {
        let _m = ConfigMockBuilder::new()
            .with_metrics(
                MetricsConfigBuilder::new()
                    .with_enabled(true)
                    .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                        enabled: true,
                        expiry_seconds: 30,
                    })
                    .build(),
            )
            .build_and_setup();

        setup_test_signer_pool();
        let _ = BalanceTracker::init().await;

        let mock_rpc = RpcMockBuilder::new().build();
        let handle = BalanceTracker::start_background_tracking(mock_rpc).await;

        assert!(handle.is_some());

        if let Some(task) = handle {
            task.abort();
        }
    }

    fn enabled_metrics_config() -> ConfigMockBuilder {
        ConfigMockBuilder::new().with_metrics(
            MetricsConfigBuilder::new()
                .with_enabled(true)
                .with_fee_payer_balance(FeePayerBalanceMetricsConfig {
                    enabled: true,
                    expiry_seconds: 30,
                })
                .build(),
        )
    }

    fn unique_signer_info() -> SignerInfo {
        SignerInfo {
            public_key: Pubkey::new_unique().to_string(),
            name: "balance_test_signer".to_string(),
            weight: 1,
            last_used: 0,
        }
    }

    fn balance_rpc(lamports: u64) -> Arc<RpcClient> {
        RpcMockBuilder::new()
            .with_custom_mock(
                RpcRequest::GetBalance,
                json!({ "context": { "slot": 1 }, "value": lamports }),
            )
            .build()
    }

    async fn update(rpc_client: &RpcClient, signer_info: &SignerInfo) -> (f64, f64) {
        let gauge_vec = SIGNER_BALANCE_GAUGES.get().unwrap();
        let fetch_errors = SIGNER_BALANCE_FETCH_ERRORS.get().unwrap();
        let pubkey = Pubkey::from_str(&signer_info.public_key).unwrap();

        BalanceTracker::update_signer_balance(
            rpc_client,
            gauge_vec,
            fetch_errors,
            signer_info,
            &pubkey,
        )
        .await;

        let labels = [signer_info.name.as_str(), signer_info.public_key.as_str()];
        (gauge_vec.with_label_values(&labels).get(), fetch_errors.with_label_values(&labels).get())
    }

    #[tokio::test]
    async fn test_update_signer_balance_sets_fetched_balance() {
        let _m = enabled_metrics_config().build_and_setup();
        BalanceTracker::init().await.unwrap();
        let signer_info = unique_signer_info();

        let (balance, errors) = update(&balance_rpc(1_850_000_000), &signer_info).await;

        assert_eq!(balance, 1_850_000_000.0);
        assert_eq!(errors, 0.0);
    }

    #[tokio::test]
    async fn test_update_signer_balance_zero_overwrites_prior_value_without_error() {
        let _m = enabled_metrics_config().build_and_setup();
        BalanceTracker::init().await.unwrap();
        let signer_info = unique_signer_info();
        update(&balance_rpc(1_000), &signer_info).await;

        let (balance, errors) = update(&balance_rpc(0), &signer_info).await;

        assert_eq!(balance, 0.0);
        assert_eq!(errors, 0.0);
    }

    #[tokio::test]
    async fn test_update_signer_balance_http_500_keeps_last_balance_and_counts_error() {
        let _m = enabled_metrics_config().build_and_setup();
        BalanceTracker::init().await.unwrap();
        let signer_info = unique_signer_info();
        update(&balance_rpc(1_850_000_000), &signer_info).await;

        let mut server = mockito::Server::new_async().await;
        let outage = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("Internal Server Error")
            .expect(2)
            .create_async()
            .await;
        let failing_rpc = RpcClient::new(server.url());

        let (balance, errors) = update(&failing_rpc, &signer_info).await;
        assert_eq!(balance, 1_850_000_000.0);
        assert_eq!(errors, 1.0);

        let (balance, errors) = update(&failing_rpc, &signer_info).await;
        assert_eq!(balance, 1_850_000_000.0);
        assert_eq!(errors, 2.0);

        outage.assert_async().await;
    }
}
