//! Prometheus metrics for RPC endpoint failover.

use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, register_int_gauge, IntCounterVec, IntGauge};

// Metric registration can only fail on a duplicate name or a malformed label, both
// of which are fixed properties of this code. Rather than panic inside a request
// path if that ever happens, an unregistered metric degrades to a no-op and the
// reason is logged.
static RPC_FAILOVERS_TOTAL: Lazy<Option<IntCounterVec>> = Lazy::new(|| {
    register_int_counter_vec!(
        "rpc_failovers_total",
        "RPC requests failed over to another endpoint after a retryable failure",
        &["endpoint_index", "reason"]
    )
    .map_err(|e| log::error!("Failed to register rpc_failovers_total: {e}"))
    .ok()
});

static RPC_ACTIVE_ENDPOINT_INDEX: Lazy<Option<IntGauge>> = Lazy::new(|| {
    register_int_gauge!("rpc_active_endpoint_index", "Index of the RPC endpoint in use")
        .map_err(|e| log::error!("Failed to register rpc_active_endpoint_index: {e}"))
        .ok()
});

/// Record that a request could not be served by `endpoint_index` and was retried
/// elsewhere. `reason` is one of `server_error`, `transport` or
/// `malformed_response`.
pub fn record_failover(endpoint_index: usize, reason: &str) {
    if let Some(counter) = RPC_FAILOVERS_TOTAL.as_ref() {
        counter.with_label_values(&[&endpoint_index.to_string(), reason]).inc();
    }
}

/// Record which endpoint is currently serving requests. `0` is the primary.
pub fn set_active_endpoint(endpoint_index: usize) {
    if let Some(gauge) = RPC_ACTIVE_ENDPOINT_INDEX.as_ref() {
        gauge.set(endpoint_index as i64);
    }
}

/// Current value of one failover counter, or zero when it has never been
/// incremented.
pub fn counter_value(endpoint_index: usize, reason: &str) -> u64 {
    RPC_FAILOVERS_TOTAL
        .as_ref()
        .and_then(|counter| {
            counter.get_metric_with_label_values(&[&endpoint_index.to_string(), reason]).ok()
        })
        .map(|metric| metric.get())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::TextEncoder;

    #[test]
    fn test_metrics_are_registered_and_render() {
        record_failover(0, "server_error");
        record_failover(1, "transport");
        set_active_endpoint(1);

        assert!(RPC_FAILOVERS_TOTAL.is_some(), "rpc_failovers_total must register");
        assert!(RPC_ACTIVE_ENDPOINT_INDEX.is_some(), "rpc_active_endpoint_index must register");

        let rendered = TextEncoder::new()
            .encode_to_string(&prometheus::gather())
            .expect("metrics should encode");

        assert!(rendered.contains("rpc_failovers_total"));
        assert!(rendered.contains("rpc_active_endpoint_index"));
    }
}
