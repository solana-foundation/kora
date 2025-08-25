# CLI Reference

*Last Updated: 2025-08-25*
> Complete reference for Kora command-line interface, including commands and flags.

## CLI Commands

| Command | Description | Example |
|:--------|:------------|:--------|
| `kora config validate` | Validate configuration file (fast, no RPC calls) | `kora --config kora.toml config validate` |
| `kora config validate-with-rpc` | Validate configuration with on-chain account verification | `kora --config kora.toml config validate-with-rpc` |
| `kora rpc start` | Start the RPC server | `kora rpc start --signers-config signers.toml` |
| `kora rpc initialize-atas` | Initialize ATAs for all payment tokens | `kora rpc initialize-atas --signers-config signers.toml` |

## Kora Flags

Customize Kora's behavior with these global command-line flags after the `kora` command:

| Flag | Description | Default | Example |
|:-----|:------------|:--------|:--------|
| `--config` | Path to Kora configuration file (TOML format) | `kora.toml` | `kora --config path/to/kora.toml` |
| `--rpc-url` | Solana RPC endpoint URL | `http://127.0.0.1:8899` | `kora --rpc-url https://api.devnet.solana.com` |
| `--help` | Print help information | - |`kora --help` |
| `--version` | Print version information | - |`kora --version` |

## RPC Server Flags

Configure the RPC server with these flags (used with `kora rpc start`):

| Flag | Description | Default | Example |
|:-----|:------------|:--------|:--------|
| `--signers-config` | Path to multi-signer configuration file (TOML) | Required* | `--signers-config signers.toml` |
| `--no-load-signer` | Skip signer initialization | `false` | `--no-load-signer` |
| `-p`, `--port` | HTTP port for RPC requests | `8080` | `--port 3000` |
| `--logging-format` | Output format for logs (`standard` or `json`) | `standard` | `--logging-format json` |
| `--help` | Print help information | - |`kora rpc start --help` |

*Required unless using `--no-load-signer`

## ATA Initialization Flags

Configure ATA initialization with these flags (used with `kora rpc initialize-atas`):

| Flag | Description | Default | Example |
|:-----|:------------|:--------|:--------|
| `--signers-config` | Path to multi-signer configuration file | Required* | `--signers-config signers.toml` |
| `--fee-payer-key` | Public key of signer to use as fee payer (must be in signers.toml) | First signer | `--fee-payer-key "pubkey123..."` |
| `--compute-unit-price` | Priority fee in micro-lamports | None | `--compute-unit-price 1000` |
| `--compute-unit-limit` | Compute unit limit for transactions | None | `--compute-unit-limit 200000` |
| `--chunk-size` | Number of ATAs to create per transaction | None | `--chunk-size 10` |

## Common Usage Examples

### Starting the RPC Server

```bash
# Basic start with default settings
kora --config path/to/kora.toml rpc start --signers-config path/to/signers.toml

# Start with custom port and config
kora --config path/to/kora.toml rpc start \
  --signers-config path/to/signers.toml \
  --port 8080 \
  --logging-format json

# Start for testing without signers
kora --config path/to/kora.toml rpc start --no-load-signer
```

### Configuration Validation

```bash
# Quick validation (offline)
kora --config path/to/kora.toml config validate

# Thorough validation with RPC checks
kora --config path/to/kora.toml --rpc-url https://api.mainnet-beta.solana.com \
  config validate-with-rpc
```

The `validate-with-rpc` command performs additional on-chain verification:
- **Program accounts**: Verifies all allowed programs exist and are executable
- **Token mints**: Confirms all allowed tokens exist as valid mint accounts
- **Payment tokens**: Validates all SPL paid tokens are valid mints
- **Payment address ATAs**: Checks if payment address has ATAs for all allowed tokens
- **Account types**: Ensures accounts have the expected type (program vs mint)

### Managing ATAs

```bash
# Initialize ATAs for payment address/signers
kora rpc initialize-atas --signers-config signers.toml

# Initialize with custom fee payer and priority
kora rpc initialize-atas \
  --signers-config signers.toml \
  --fee-payer-key "7xKXtg2CW87d3HEQ2BpKHpcPKBhpKGQPPRQJyccVLow9" \
  --compute-unit-price 1000 \
  --chunk-size 10
```

## Environment Variables

These environment variables can be used instead of command-line flags:

| Variable | Description | Flag Equivalent |
|:---------|:------------|:----------------|
| `RPC_URL` | Solana RPC endpoint | `--rpc-url` |


## See Also

* [Operators Guide](README.md) - Overview of Kora operators
* [Configuration Guide](CONFIGURATION.md) - Detailed configuration options
* [Signers Guide](SIGNERS.md) - Signer types and configuration
* [Authentication Guide](AUTHENTICATION.md) - Setting up API authentication
* [Quick Start Guide](/getting-started/QUICK_START.md) - Getting started with Kora