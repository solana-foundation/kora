pub mod balance;
pub mod handler;
pub mod middleware;

pub use balance::BalanceTracker;
pub use handler::{MetricsHandlerLayer, MetricsHandlerService};
pub use middleware::{HttpMetrics, HttpMetricsLayer, HttpMetricsService};
pub use prometheus;
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::task::JoinHandle;

use crate::config::MetricsConfig;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;
use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    RpcModule,
};
use prometheus::{Encoder, TextEncoder};
use std::{collections::HashSet, net::SocketAddr, sync::Arc};

pub struct MetricsLayers {
    pub http_metrics_layer: Option<HttpMetricsLayer>,
    pub metrics_handler_layer: Option<MetricsHandlerLayer>,
}

fn get_allowed_methods() -> Arc<HashSet<String>> {
    let methods = match get_config() {
        Ok(c) => c.kora.enabled_methods.get_enabled_method_names(),
        Err(e) => {
            log::warn!("Failed to load config for metrics allowed methods: {}", e);
            vec![]
        }
    };
    Arc::new(HashSet::from_iter(methods))
}

fn get_metrics_layers(metrics_config: &MetricsConfig) -> Option<MetricsLayers> {
    if metrics_config.enabled {
        Some(MetricsLayers {
            http_metrics_layer: Some(HttpMetricsLayer::new(get_allowed_methods())),
            metrics_handler_layer: Some(MetricsHandlerLayer::new(metrics_config.endpoint.clone())),
        })
    } else {
        None
    }
}

pub async fn run_metrics_server_if_required(
    rpc_port: u16,
    rpc_client: Arc<RpcClient>,
) -> Result<(Option<ServerHandle>, Option<MetricsLayers>, Option<JoinHandle<()>>), anyhow::Error> {
    let metrics_config = get_config()?.metrics.clone();

    if !metrics_config.enabled {
        return Ok((None, None, None));
    }

    let balance_tracker_handle = if let Err(e) = BalanceTracker::init().await {
        log::warn!("Failed to initialize balance tracker: {e}");
        // A failed balance tracker must not block metrics server startup.
        None
    } else {
        BalanceTracker::start_background_tracking(rpc_client).await
    };

    // Sharing the RPC port means no separate metrics server is needed.
    if metrics_config.port == rpc_port {
        log::info!("Metrics endpoint enabled at {} on RPC server", metrics_config.endpoint);
        return Ok((None, get_metrics_layers(&metrics_config), balance_tracker_handle));
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], metrics_config.port));
    log::info!("Metrics server started on {addr}, port {}", metrics_config.port);
    log::info!("Metrics endpoint: {}", metrics_config.endpoint);

    let middleware = tower::ServiceBuilder::new()
        .layer(MetricsHandlerLayer::new(metrics_config.endpoint.clone()));

    let server =
        ServerBuilder::default().set_middleware(middleware).http_only().build(addr).await?;

    let module = RpcModule::new(());

    let metrics_server_handle = server
        .start(module)
        .map_err(|e| anyhow::anyhow!("Failed to start metrics server: {}", e))?;

    // Return both the metrics server handle AND the HTTP metrics middleware for the main RPC server
    // The HTTP middleware needs to be on the RPC server to collect metrics, even if metrics are served separately
    let metrics_layers = MetricsLayers {
        http_metrics_layer: Some(HttpMetricsLayer::new(get_allowed_methods())), // Collect metrics on RPC server
        metrics_handler_layer: None, // Don't serve metrics on RPC server (separate server handles this)
    };

    Ok((Some(metrics_server_handle), Some(metrics_layers), balance_tracker_handle))
}

/// Gather all Prometheus metrics and encode them in text format
pub fn gather() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    String::from_utf8(buffer).map_err(Into::into)
}
