use prometheus::{Encoder, TextEncoder};

// Re-export prometheus for use in other crates
pub use prometheus;

// Export the HTTP middleware
pub mod middleware;
pub use middleware::{HttpMetricsLayer, HttpMetricsService};

// Export the metrics handler
pub mod handler;
pub use handler::{MetricsHandlerLayer, MetricsHandlerService};

/// Gather all Prometheus metrics and encode them in text format
pub fn gather() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    String::from_utf8(buffer).map_err(Into::into)
}
