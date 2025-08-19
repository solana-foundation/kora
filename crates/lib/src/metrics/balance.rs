use crate::{cache::CacheUtil, error::KoraError, get_request_signer, state::get_config};
use prometheus::{register_gauge, Gauge};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::{
    sync::OnceCell,
    task::JoinHandle,
    time::{interval, Duration},
};

/// Global Prometheus gauge for tracking fee payer balance
static FEE_PAYER_BALANCE_GAUGE: OnceCell<Gauge> = OnceCell::const_new();

/// Balance tracker for monitoring signer SOL balance
pub struct BalanceTracker;

impl BalanceTracker {
    /// Initialize the Prometheus gauge for balance tracking
    pub async fn init() -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        let gauge = register_gauge!(
            "fee_payer_balance_lamports",
            "Current SOL balance of the fee payer/signer in lamports"
        )
        .map_err(|e| {
            KoraError::InternalServerError(format!("Failed to register balance gauge: {e}"))
        })?;

        FEE_PAYER_BALANCE_GAUGE.set(gauge).map_err(|_| {
            KoraError::InternalServerError("Balance gauge already initialized".to_string())
        })?;

        log::info!("Balance tracking metrics initialized");
        Ok(())
    }

    /// Track the current signer balance and update Prometheus metrics
    pub async fn track_signer_balance(rpc_client: &Arc<RpcClient>) -> Result<(), KoraError> {
        if !BalanceTracker::is_enabled() {
            return Ok(());
        }

        // TODO
        // Get the signer and extract pubkey
        let signer = get_request_signer()?;
        let signer_pubkey = signer.solana_pubkey();

        // Get account balance using cache with configured expiry
        let account = CacheUtil::get_account(rpc_client, &signer_pubkey, false).await?;
        let balance_lamports = account.lamports;

        // Update Prometheus gauge
        if let Some(gauge) = FEE_PAYER_BALANCE_GAUGE.get() {
            gauge.set(balance_lamports as f64);

            log::debug!(
                "Updated fee payer balance metrics: {balance_lamports} lamports for signer {signer_pubkey}"
            );
        } else {
            log::warn!("Balance gauge not initialized, skipping metrics update");
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
        log::info!("Starting balance tracking background task with {interval_seconds}s interval");

        // Spawn a background task that runs forever
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                // Track balance, but don't let errors crash the loop
                if let Err(e) = BalanceTracker::track_signer_balance(&rpc_client).await {
                    log::warn!("Failed to track signer balance in background task: {e}");
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
        state::update_config,
    };
    use serial_test::serial;

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
}
