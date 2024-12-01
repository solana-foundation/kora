# Kora

A paymaster node that provides a JSON-RPC interface.

## Features

- JSON-RPC server with middleware support
- Transaction fee estimation
- Health check endpoint (`/liveness`)
- Configurable logging (JSON or standard format)
- CORS support
- Customizable RPC endpoint

## Usage

```bash
cargo run -- [OPTIONS]
```

### Command Line Arguments

```
Options:
  -p, --port <PORT>
          Port number for the RPC server [default: 8080]

      --rpc-url <RPC_URL>
          RPC URL to connect to [env: RPC_URL=] [default: http://127.0.0.1:8899]

      --logging-format <FORMAT>
          Logging format (standard or json) [default: standard]

      --metrics-endpoint <ENDPOINT>
          Optional metrics endpoint URL

      --private-key <PRIVATE_KEY>
          Base58-encoded private key for signing [env: KORA_PRIVATE_KEY=]
          Required unless --no-load-signer is used

      --config <FILE>
          Path to kora.toml config file [default: kora.toml]

      --no-load-signer
          Skip loading the signer

  -h, --help
          Print help information
```

### Environment Variables

- `RUST_LOG`: Controls log level and filtering (e.g., "info,sqlx=error,sea_orm_migration=error,jsonrpsee_server=warn")
- `RPC_URL`: Alternative way to specify the RPC URL
- `KORA_PRIVATE_KEY`: Alternative way to specify the signing private key

### Kora.toml config

The `kora.toml` config file is used to configure the paymaster node.

```toml
[features]
enabled = ["gasless"]

[tokens]
allowed = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",  # USDT
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",  # BONK,
    "So11111111111111111111111111111111111111112",  # SOL
]
```

## RPC Methods

### estimateTransactionFee

Estimates the transaction fee for a given Solana transaction.

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "estimateTransactionFee",
    "params": [
        "<base58-encoded-transaction>"
    ]
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "fee": "<estimated-fee-in-lamports>"
    }
}
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Linting

```bash
# Run clippy with warnings as errors
make lint

# Run clippy with auto-fix
make lint-fix-all
```

### Running the Server

1. Start the server with default settings:
   ```bash
   cargo run
   ```

2. Start with custom port and RPC URL:
   ```bash
   cargo run -- --port 9000 --rpc-url http://localhost:8899
   ```

3. Enable JSON logging:
   ```bash
   cargo run -- --logging-format json
   ```
