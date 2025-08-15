# Kora Metrics Guide

The [Kora Metrics Crate](../../crates/metrics) provides comprehensive metrics collection and monitoring for the Kora RPC server.

Kora exposes a `/metrics` endpoint that provides real-time performance data in Prometheus format.

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

3. **Prometheus Scraping** - Configured to scrape Kora every 60 seconds (see [`crates/metrics/prometheus.yml`](../../crates/metrics/prometheus.yml)):
   ```yaml
   scrape_configs:
     - job_name: 'kora'
       static_configs:
         - targets: ['kora:8080']
   ```