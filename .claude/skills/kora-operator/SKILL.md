---
name: kora-operator
description: "Kora paymaster node operator guide. Use when the user asks about: configuring kora.toml (rate limits, validation, allowed programs/tokens, fee payer policy, pricing, auth, caching), setting up signers.toml (memory/Turnkey/Privy/Vault, pool strategies), running Kora (kora rpc start, config validate, CLI), deploying to Docker/Railway, fee calculation (margin/fixed/free pricing), Prometheus monitoring, or API key/HMAC authentication setup. Not for client SDK integration (use kora-client)."
---

# Kora Node Operator Guide

Run Kora nodes to validate, sign, and sponsor Solana transaction fees for your users.

**Docs**: https://launch.solana.com/docs/kora/operators
**Install**: `cargo install kora-cli`
**Source**: https://github.com/solana-foundation/kora

## Quick Start

```bash
# Install
cargo install kora-cli

# Minimal config files needed: kora.toml + signers.toml

# Validate config
kora --config kora.toml config validate

# Start server
kora --config kora.toml rpc start --signers-config signers.toml

# Initialize payment ATAs (for receiving token payments)
kora --config kora.toml rpc initialize-atas --signers-config signers.toml
```

## CLI Reference

```bash
# Global flags
kora --config <path>     # kora.toml location (default: ./kora.toml)
kora --rpc-url <url>     # Solana RPC URL (overrides config, env: RPC_URL)

# Commands
kora config validate                    # Validate kora.toml
kora config validate-with-rpc           # Validate with live RPC calls
kora rpc start --signers-config <path>  # Start RPC server
kora rpc start --no-load-signer         # Start without loading signers (limited functionality)
kora rpc start --port 8080              # Custom port (default: 8080)
kora rpc start --logging-format json    # JSON logging (default: standard)
kora rpc initialize-atas --signers-config <path>  # Initialize payment ATAs

# ATA init options
--fee-payer-key <base58>     # Custom fee payer for ATA creation
--compute-unit-price <num>   # Priority fee
--compute-unit-limit <num>   # Compute budget
--chunk-size <num>           # Batch size for ATA creation
```

## Configuration Overview

Two config files required:

| File | Purpose |
|------|---------|
| `kora.toml` | Server config: validation, auth, pricing, caching, methods, metrics |
| `signers.toml` | Signer pool: keys, types, selection strategy |

### Minimal kora.toml

```toml
[kora]
rate_limit = 100

[validation]
max_allowed_lamports = 1000000
max_signatures = 10
price_source = "Mock"  # or "Jupiter" (requires JUPITER_API_KEY env var)

allowed_programs = [
    "11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    "ComputeBudget111111111111111111111111111111",
]

allowed_tokens = ["<your-token-mint>"]
allowed_spl_paid_tokens = ["<your-token-mint>"]

[validation.price]
type = "margin"
margin = 0.1  # 10% markup
```

### Minimal signers.toml

```toml
[signer_pool]
strategy = "round_robin"  # or "random", "weighted"

[[signers]]
name = "signer_1"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY"
weight = 1
```

## Detailed References

- **Full kora.toml reference**: See [references/configuration.md](references/configuration.md) - all sections, fields, and production examples
- **Signer types and setup**: See [references/signers.md](references/signers.md) - memory, Turnkey, Privy, Vault configuration
- **Fee calculation and pricing**: See [references/fees.md](references/fees.md) - fee components, pricing models, security considerations

## Key Operator Decisions

### Pricing Model

| Model | Use Case | Security |
|-------|----------|----------|
| `margin` (default) | Charge users cost + markup | Safest - includes fee payer outflow |
| `fixed` | Flat fee per transaction | Must disable fee payer transfers in policy |
| `free` | Sponsor all fees | Must disable fee payer transfers in policy |

### Authentication

Both optional, can be used simultaneously:
- **API Key**: Simple `x-api-key` header. Set `api_key` in `[kora.auth]` or `KORA_API_KEY` env var.
- **HMAC**: Cryptographic signature with timestamp. Set `hmac_secret` in `[kora.auth]` or `KORA_HMAC_SECRET` env var.

### Fee Payer Policy

Control what actions the fee payer can perform in transactions. All default to `false` (restrictive). Explicitly set to `true` only what you need.

Critical for `fixed`/`free` pricing: ensure `allow_transfer` remains `false` (the default) on system and token programs to prevent fee payer fund drain.

### Caching

Optional Redis caching for token account lookups:
```toml
[kora.cache]
enabled = true
url = "redis://localhost:6379"
default_ttl = 300
account_ttl = 60
```

### Monitoring

Optional Prometheus metrics at `/metrics`:
```toml
[metrics]
enabled = true
endpoint = "/metrics"
port = 9090
```

Key metrics: `kora_http_requests_total`, `kora_http_request_duration_seconds`, `signer_balance_lamports`.

## Deployment

### Docker

Pre-built image available:
```bash
docker pull ghcr.io/solana-foundation/kora:v2.0.4
docker run -v ./kora.toml:/kora.toml -v ./signers.toml:/signers.toml \
  -e RPC_URL=https://api.mainnet-beta.solana.com \
  -e KORA_PRIVATE_KEY=<key> \
  -p 8080:8080 \
  ghcr.io/solana-foundation/kora:v2.0.4
```

Or build from source:
```dockerfile
FROM rust:1.86-bookworm AS builder
RUN cargo install kora-cli
FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/kora /usr/local/bin/kora
COPY kora.toml signers.toml ./
ENV RPC_URL=https://api.mainnet-beta.solana.com
CMD ["kora", "rpc", "start", "--signers-config", "signers.toml"]
```

### Railway

1. Create project with kora.toml, signers.toml, Dockerfile
2. `railway login && railway init && railway up`
3. Set env vars in dashboard: `RPC_URL`, `KORA_PRIVATE_KEY`, `RUST_LOG`
4. Generate public domain from settings

**Docs**: https://launch.solana.com/docs/kora/operators/deployment/railway

## Kora Core Development

For contributors working on the Kora codebase itself:

```bash
make build          # Build all crates
make build-lib      # Build kora-lib only
make build-cli      # Build kora-cli only
make check          # Check formatting
make fmt            # Format + lint with auto-fix
make unit-test      # Run unit tests
make integration-test  # Run full integration test suite
```
