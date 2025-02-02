# Kora

Kora is a Solana paymaster node that provides a JSON-RPC interface for handling gasless transactions and fee abstractions. It enables developers to build applications where users can pay transaction fees in tokens other than SOL.

## Features

- JSON-RPC server with middleware support
- Transaction fee estimation in any supported token
- Gasless transaction support
- Transfer transactions with any supported token
- Remote signer support
- Disallowed account, allowed spl tokens, programs config

> Note: only `signAndSend` submits a transaction to an RPC, all other methods only return a signed transaction

## Crates

- `kora-lib`: Shared library for kora
- `kora-rpc`: JSON-RPC server
- `kora-cli`: Command line interface for kora

## Getting Started

### Installation

```bash
git clone https://github.com/yourusername/kora.git
cd kora
make install
```

### Running the Server

Basic usage:
```bash
kora -- [OPTIONS]
```

### Configuration

#### Command Line Arguments

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --port <PORT>` | Port number for the RPC server | 8080 |
| `--rpc-url <RPC_URL>` | RPC URL to connect to | http://127.0.0.1:8899 |
| `--logging-format <FORMAT>` | Logging format (standard or json) | standard |
| `--metrics-endpoint <ENDPOINT>` | Optional metrics endpoint URL | - |
| `--private-key <PRIVATE_KEY>` | Base58-encoded private key for signing | - |
| `--config <FILE>` | Path to kora.toml config file | kora.toml |
| `--no-load-signer` | Skip loading the signer | false |
| `--with-turnkey-signer` | Use Turnkey signer | false |
| `--turnkey-api-public-key <TURNKEY_API_PUBLIC_KEY>` | Turnkey API public key | - |
| `--turnkey-api-private-key <TURNKEY_API_PRIVATE_KEY>` | Turnkey API private key | - |
| `--turnkey-organization-id <TURNKEY_ORGANIZATION_ID>` | Turnkey organization ID | - |
| `--turnkey-private-key-id <TURNKEY_PRIVATE_KEY_ID>` | Turnkey private key ID | - |
| `--turnkey-public-key <TURNKEY_PUBLIC_KEY>` | Turnkey public key | - |

#### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `RUST_LOG` | Controls log level and filtering | "info,sqlx=error" |
| `RPC_URL` | Alternative way to specify the RPC URL | - |
| `KORA_PRIVATE_KEY` | Alternative way to specify the signing private key | - |
| `TURNKEY_API_PUBLIC_KEY` | Turnkey API public key | - |
| `TURNKEY_API_PRIVATE_KEY` | Turnkey API private key | - |
| `TURNKEY_ORGANIZATION_ID` | Turnkey organization ID | - |
| `TURNKEY_PRIVATE_KEY_ID` | Turnkey private key ID | - |
| `TURNKEY_PUBLIC_KEY` | Turnkey public key | - |
| `TEST_SENDER_PUBKEY` | Test sender pubkey | - |
| `TEST_SENDER_MNEMONIC` | Test sender mnemonic | - |

#### Configuration File (kora.toml)

The `kora.toml` file configures the paymaster node's features and supported tokens:

```toml
[validation]
max_allowed_lamports = 1000000
max_signatures = 10
allowed_programs = [
    "11111111111111111111111111111111",  # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  # Token Program
]
allowed_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",  # USDT
    "So11111111111111111111111111111111111111112",  # SOL
]
allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",  # USDT
]
disallowed_accounts = []
```

## API Reference

### RPC Methods

#### `estimateTransactionFee`

Estimates the transaction fee in terms of a specified token.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "estimateTransactionFee",
    "params": [
        {
            "transaction": "<base58-encoded-transaction>",
            "fee_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        }
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "fee_in_lamports": "1000000"
    }
}
```


#### `getSupportedTokens`

Returns supported token addresses.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getSupportedTokens",
    "params": []
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "tokens": [
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
            "So11111111111111111111111111111111111111112"
        ]
    }
}
```

#### `signTransaction`

Signs a transaction with the paymaster's key.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "signTransaction",
    "params": [
        {
            "transaction": "<base58-encoded-transaction>"
        }
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<base58-encoded-signed-transaction>"
    }
}
```

#### `signAndSend`

Signs and submits a transaction to the network.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "signAndSend",
    "params": [
        {
            "transaction": "<base58-encoded-transaction>"
        }
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<base58-encoded-signed-transaction>"
    }
}
```

#### `transactionTransfer`

Create a transfer request and sign as the paymaster (SPL and SOL)

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "transactionTransfer",
    "params": [
        1000000, // lamports value
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // mint address
        "5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht", // source
        "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM" // recipient
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "transaction": "<base58-encoded-transaction>",
        "message": "<message>",
        "blockhash": "<base58-encoded-blockhash>"
    }
}
```

#### `getBlockhash`

Returns the latest blockhash.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlockhash",
    "params": []
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "blockhash": "<base58-encoded-blockhash>"
    }
}
```

#### `getConfig`

Returns the paymaster configuration.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getConfig",
    "params": []
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "fee_payer": "<payer-pubkey>",
        "validation_config": {
            "max_allowed_lamports": 1000000,
            "max_signatures": 10,
            "allowed_programs": ["11111111111111111111111111111111", "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"],
            "allowed_tokens": ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "So11111111111111111111111111111111111111112"]
        }
    }
}
```

<!-- #### `swapToSol`

Creates a swap request to SOL and returns a signed transaction.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "swapToSol",
    "params": [
        "<account-pubkey>",
        100,
        "<token-mint-address>"
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "transaction": "<base58-encoded-transaction>"
    }
}
``` -->

#### `signTransactionIfPaid`

Signs a transaction if the user has paid the required amount of tokens.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "signTransactionIfPaid",
    "params": [
        {
            "transaction": "<base58-encoded-transaction>",
            "cost_in_lamports": 1000000,
            "margin": 0.0
        }
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<base58-encoded-signed-transaction>"
    }
}
```

## Development

### Prerequisites

- Rust 1.70 or later
- Solana CLI tools (for testing)
- A Solana RPC endpoint (local or remote)

### Building

```bash
# Build
make build
```

### Testing

```bash
# Run all tests
make test

# Run integration tests
make test-integrations
```

### Running

```bash
# Basic
kora -- \
    --rpc-url <RPC_URL> \
    --port <PORT> 

# With Turnkey (or use environment variables)
kora -- \
    --rpc-url <RPC_URL> \
    --port <PORT> \
    --with-turnkey-signer \
    --turnkey-api-public-key <TURNKEY_API_PUBLIC_KEY> \
    --turnkey-api-private-key <TURNKEY_API_PRIVATE_KEY> \
    --turnkey-organization-id <TURNKEY_ORGANIZATION_ID> \
    --turnkey-private-key-id <TURNKEY_PRIVATE_KEY_ID> \
    --turnkey-public-key <TURNKEY_PUBLIC_KEY>

# No signer
kora -- \
    --rpc-url <RPC_URL> \
    --port <PORT> \
    --no-load-signer

# Load private key at runtime without .env
kora -- \
    --rpc-url <RPC_URL> \
    --port <PORT> \
    --private-key <PRIVATE_KEY>
```

### Code Quality

```bash
# Run clippy with warnings as errors
make lint

# Run clippy with auto-fix
make lint-fix-all

# Format code
make fmt
```

### Local Development

1. Start a local Solana validator:
   ```bash
   solana-test-validator
   ```

2. Run the server with development settings:
   ```bash
   RUST_LOG=debug cargo run -- --port 9000 --rpc-url http://localhost:8899
   ```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
