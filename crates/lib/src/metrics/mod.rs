pub mod handler;
pub mod middleware;

pub use handler::{MetricsHandlerLayer, MetricsHandlerService};
pub use middleware::{HttpMetricsLayer, HttpMetricsService};
pub use prometheus;

use crate::{config::MetricsConfig, state::get_config};
use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    RpcModule,
};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

pub struct MetricsLayers {
    pub http_metrics_layer: Option<HttpMetricsLayer>,
    pub metrics_handler_layer: Option<MetricsHandlerLayer>,
}

fn get_metrics_layers(metrics_config: &MetricsConfig) -> Option<MetricsLayers> {
    if metrics_config.enabled {
        Some(MetricsLayers {
            http_metrics_layer: Some(HttpMetricsLayer::new()),
            metrics_handler_layer: Some(MetricsHandlerLayer::new(metrics_config.endpoint.clone())),
        })
    } else {
        None
    }
}

pub async fn run_metrics_server_if_required(
    rpc_port: u16,
) -> Result<(Option<ServerHandle>, Option<MetricsLayers>), anyhow::Error> {
    let metrics_config = get_config()?.metrics.clone();

    if !metrics_config.enabled {
        return Ok((None, None));
    }

    // If running on the same port as the RPC server, we don't need to run a separate metrics server
    if metrics_config.port == rpc_port {
        log::info!("Metrics endpoint enabled at {} on RPC server", metrics_config.endpoint);
        return Ok((None, get_metrics_layers(&metrics_config)));
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], metrics_config.port));
    log::info!("Metrics server started on {addr}, port {}", metrics_config.port);
    log::info!("Metrics endpoint: {}", metrics_config.endpoint);

    // Simple middleware stack for metrics-only server
    let middleware = tower::ServiceBuilder::new()
        .layer(MetricsHandlerLayer::new(metrics_config.endpoint.clone()));

    // Configure and build the server
    let server =
        ServerBuilder::default().set_middleware(middleware).http_only().build(addr).await?;

    // Empty RPC module since we only serve metrics
    let module = RpcModule::new(());

    let metrics_server_handle = server
        .start(module)
        .map_err(|e| anyhow::anyhow!("Failed to start metrics server: {}", e))?;

    // Return both the metrics server handle AND the HTTP metrics middleware for the main RPC server
    // The HTTP middleware needs to be on the RPC server to collect metrics, even if metrics are served separately
    let metrics_layers = MetricsLayers {
        http_metrics_layer: Some(HttpMetricsLayer::new()), // Collect metrics on RPC server
        metrics_handler_layer: None, // Don't serve metrics on RPC server (separate server handles this)
    };

    Ok((Some(metrics_server_handle), Some(metrics_layers)))
}

/// Gather all Prometheus metrics and encode them in text format
pub fn gather() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    String::from_utf8(buffer).map_err(Into::into)
}
