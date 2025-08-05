# Kora Metrics

This crate provides comprehensive metrics collection and monitoring for the Kora RPC server.

## Metrics Exported

### HTTP Metrics
- `kora_http_requests_total{method, status}` - Counter of HTTP requests by JSON-RPC method and status code
- `kora_http_request_duration_seconds{method}` - Histogram of request durations by JSON-RPC method

## Monitoring Stack

### Prometheus Configuration
- `prometheus.yml` - Prometheus scraping configuration

### Grafana Dashboard
- `grafana/provisioning/datasources/prometheus.yml` - Auto-configures Prometheus data source
- `grafana/provisioning/dashboards/kora-metrics.json` - Pre-built dashboard with:
  - HTTP Request Rate
  - Response Time Percentiles (95th/50th)
  - Total Request Counter
  - Request Distribution by Method