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

  -r, --rpc-url <RPC_URL>
          RPC URL to connect to [default: "http://127.0.0.1:8899"]

  -l, --logging-format <FORMAT>
          Logging format (standard or json) [default: "standard"]

      --metrics-endpoint <ENDPOINT>
          Optional metrics endpoint URL

      --private-key <PRIVATE_KEY>
          Base58-encoded private key for signing

  -h, --help
          Print help information
```

### Environment Variables

- `RUST_LOG`: Controls log level and filtering (e.g., "info,sqlx=error,sea_orm_migration=error,jsonrpsee_server=warn")
- `RPC_URL`: Alternative way to specify the RPC URL
- `KORA_PRIVATE_KEY`: Alternative way to specify the signing private key

## RPC Methods

### estimateTransactionFee

Estimates the transaction fee for a given Solana transaction.

Request:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "estimateTransactionFee",
    "params": {
        "transaction_data": "<base58-encoded-transaction>"
    }
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
make lint-fix
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
