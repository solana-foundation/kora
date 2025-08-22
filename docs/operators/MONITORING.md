# Kora Metrics Guide

*Last Updated: 2025-08-22*

The [Kora Metrics Crate](../../crates/metrics) provides comprehensive metrics collection and monitoring for the Kora RPC server.

Kora exposes a `/metrics` endpoint that provides real-time performance data in Prometheus format.

## Configuration

Metrics are configured in the `[metrics]` section of your `kora.toml`. The `[metrics]` section configures metrics collection and monitoring. This section is optional and by default, metrics are disabled.

```toml
[metrics]
enabled = true
endpoint = "/metrics"
port = 8080
scrape_interval = 60

[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 30
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable metrics collection | ✅ | boolean |
| `endpoint` | Custom metrics endpoint path | ✅ | string |
| `port` | Metrics endpoint port | ✅ | number |
| `scrape_interval` | Frequency of Prometheus scrape (seconds) | ✅ | number |

### Fee Payer Balance Tracking

The `[metrics.fee_payer_balance]` section configures automatic monitoring of your fee payer's SOL balance:

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable fee payer balance tracking | ❌ (default: false) | boolean |
| `expiry_seconds` | Background tracking interval in seconds | ❌ (default: 30) | number |

When enabled, Kora automatically tracks your fee payer's SOL balance and exposes it via the `fee_payer_balance_lamports` Prometheus gauge. This helps with capacity planning and low-balance alerting.



## Quick Start

**Access metrics:**
```bash
curl http://localhost:8080/metrics
```

## What You'll See

The metrics show how your RPC server is performing:

```
# Total requests by method and status
kora_http_requests_total{method="signTransaction",status="200"} 42
kora_http_requests_total{method="signTransaction",status="400"} 3

# Request duration (in seconds) by method  
kora_http_request_duration_seconds{method="signTransaction"} 0.045

# Signer balances (for multi-signer setups)
signer_balance_lamports{signer_name="primary_signer",signer_pubkey="4gBe...xyz"} 500000000
signer_balance_lamports{signer_name="backup_signer",signer_pubkey="7XyZ...abc"} 300000000
```

If you haven't called the RPC server yet, you will not see any metrics. You can run a simple test by calling the `getConfig` method:

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "getConfig", "id": 1}'
```

and then check the metrics:

```bash
curl http://localhost:8080/metrics
```

## Key Metrics Explained

1. **`kora_http_requests_total`** - How many requests you've handled
   - `method`: Which RPC method was called
   - `status`: HTTP status code (200=success, 400=error, etc.)
   - Use this to track usage patterns and error rates

2. **`kora_http_request_duration_seconds`** - How fast your requests are
   - Shows percentiles (p50, p95, p99) for response times
   - Use this to identify slow endpoints

3. **`signer_balance_lamports`** - Current SOL balance of each signer
   - Shows balance in lamports (1 SOL = 1,000,000,000 lamports) for each signer
   - Labels: `signer_name` (human-readable name) and `signer_pubkey` (public key)
   - Updated automatically in the background when enabled
   - Use this for capacity planning and low-balance alerts across all signers

## Using the Data

### Option 1: Quick Health Check
```bash
# See all metrics
curl http://localhost:8080/metrics

# Check specific method performance
curl http://localhost:8080/metrics | grep signTransaction
```

### Option 2: Prometheus + Grafana (Recommended)
For graphs and alerts, run the full monitoring stack:

```bash
# from kora root directory
make run-metrics
```

Then visit:
- Prometheus: http://localhost:9090 (query metrics)
- Grafana Pre-built Kora dashboard: http://localhost:3000
  - Default login: admin/admin (or use the `GF_SECURITY_ADMIN_PASSWORD` and `GF_SECURITY_ADMIN_USER` credentials from your `.env` file)

### Option 3: Your Own Monitoring
Point any Prometheus-compatible tool at `http://your-server:8080/metrics`:
- Datadog
- New Relic  
- CloudWatch
- VictoriaMetrics

## Example Queries (Prometheus)

```promql
# Requests per second by method
rate(kora_http_requests_total[1m])

# 95th percentile response time
histogram_quantile(0.95, kora_http_request_duration_seconds_bucket)

# Error rate
rate(kora_http_requests_total{status!="200"}[5m])

# Balance of specific signer by name
signer_balance_lamports{signer_name="primary_signer"} / 1000000000

# Balance of specific signer by public key
signer_balance_lamports{signer_pubkey="4gBe...xyz"} / 1000000000

# Total balance across all signers
sum(signer_balance_lamports) / 1000000000

# Minimum balance across all signers (useful for alerts)
min(signer_balance_lamports) / 1000000000
```

## Multi-Signer Monitoring

When using multiple signers, you can monitor each signer individually or track aggregate metrics:

### Individual Signer Metrics
```bash
# Check balance of specific signer
curl http://localhost:8080/metrics | grep 'signer_balance_lamports{signer_name="primary_signer"}'

# View all signer balances
curl http://localhost:8080/metrics | grep signer_balance_lamports
```

### Prometheus Queries for Multi-Signer Setups
```promql
# Alert if any signer has low balance (< 0.05 SOL)
min(signer_balance_lamports) < 50000000

# Monitor balance distribution across signers
signer_balance_lamports / on() group_left() sum(signer_balance_lamports)

# Track signer with lowest balance
min_over_time(signer_balance_lamports[1h])

# Count number of healthy signers (> 0.01 SOL)
count(signer_balance_lamports > 10000000)

# Average balance across all signers
avg(signer_balance_lamports) / 1000000000
```

## Security Note

The `/metrics` endpoint is public by default. In production, consider:
- Putting it behind a firewall
- Using a separate metrics port
- Adding authentication via reverse proxy

## How Metrics Collection Works

1. **HTTP Middleware Layer** - Intercepts all requests and collects:
   - Request count by JSON-RPC method and status
   - Request duration by method

2. **Metrics Endpoint** - `/metrics` endpoint exposed automatically when feature is enabled
   - Handled by `MetricsHandlerLayer` 
   - Returns Prometheus-formatted metrics

3. **Prometheus Scraping** - Configured to scrape Kora every 60 seconds (see [`crates/lib/src/metrics/prometheus.yml`](/crates/lib/src/metrics/prometheus.yml))