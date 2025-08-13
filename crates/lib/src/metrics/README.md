# Kora Metrics

This crate provides comprehensive metrics collection and monitoring for the Kora RPC server.

## Configuration

Metrics are configured in the main `kora.toml` file:

```toml
[metrics]
enabled = true           # Enable/disable metrics collection
endpoint = "/metrics"    # HTTP endpoint for Prometheus scraping
port = 8080             # Port for metrics (same as RPC or separate)
scrape_interval = 60    # Prometheus scrape interval in seconds
```

## Auto-Configuration

The metrics Docker stack (Prometheus + Grafana) automatically reads configuration from `kora.toml`:

```bash
# Update prometheus.yml and docker-compose.metrics.yml from kora.toml
make update-metrics-config

# Or run metrics (automatically updates config first)
make run-metrics
```

### Manual Configuration Update

You can also run the update utility directly:

```bash
cd crates/metrics
cargo run --bin update-config
```

This utility will:
1. Read metrics configuration from `../../kora.toml`
2. Update `prometheus.yml` with the correct port, endpoint, and scrape interval
3. **Preserve custom modifications** in existing files
4. Display the configuration that was applied

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

## Running Metrics

### Same Port as RPC Server
When `port = 8080` (same as RPC server), metrics are served on the main RPC server at `http://localhost:8080/metrics`.

### Separate Port
When `port = 9090` (different from RPC server), a dedicated metrics server runs on the specified port at `http://localhost:9090/metrics`.

## Docker Compose Stack

Start Prometheus and Grafana:

```bash
make run-metrics
```

This will:
- Automatically update configuration from kora.toml
- Start Prometheus on port 9090
- Start Grafana on port 3000
- Configure Prometheus to scrape Kora metrics

Access:
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (default login: admin/admin)

## Additional Documentation

- [Kora Monitoring Guide](../../docs/operators/MONITORING.md)