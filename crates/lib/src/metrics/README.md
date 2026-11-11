# Kora Metrics

This crate provides comprehensive metrics collection and monitoring for the Kora RPC server.

## Configuration

Metrics are configured in the main `kora.toml` file:

```toml
[metrics]
enabled = true           # Enable/disable metrics collection
endpoint = "/metrics"    # HTTP endpoint for Prometheus scraping
port = 8080             # Port for metrics (same as RPC or separate)
```

## Prometheus Configuration

The metrics Docker stack (Prometheus + Grafana) scrapes the target set in `prometheus.yml`, which
defaults to `host.docker.internal:8080` at `/metrics`. If you change `port` or `endpoint` in
`kora.toml`, update the `kora` job in `prometheus.yml` to match.

## Metrics Exported

### HTTP Metrics
- `kora_http_requests_total{method, status}` - Counter of HTTP requests by JSON-RPC method and status code
- `kora_http_request_duration_seconds{method}` - Histogram of request durations by JSON-RPC method
- `kora_http_rejections_total{method, reason}` - Counter of rejected HTTP requests by method and rejection reason (e.g. `auth_failure`, `rate_limit`)

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

## Running Metrics

### Same Port as RPC Server
When `port = 8080` (same as RPC server), metrics are served on the main RPC server at `http://localhost:8080/metrics`.

### Separate Port
When `port = 9090` (different from RPC server), a dedicated metrics server runs on the specified port at `http://localhost:9090/metrics`.

## Docker Compose Stack

Start Prometheus and Grafana:

```bash
just run-metrics
```

This will:
- Start Prometheus on port 9090
- Start Grafana on port 3000
- Configure Prometheus to scrape Kora metrics

Access:
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (default login: admin/admin)

## Additional Documentation

- [Kora Monitoring Guide](https://launch.solana.com/docs/kora/operators/monitoring)