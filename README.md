# Kora

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/solana-foundation/kora)

Kora is a Solana paymaster node that provides a JSON-RPC interface for handling gasless transactions and fee abstractions. It enables developers to build applications where users can pay transaction fees in tokens other than SOL.

## Features

- JSON-RPC server with middleware support
- Transaction fee estimation in any supported token
- Gasless transaction support
- Transfer transactions with any supported token
- Remote signer support
- Disallowed account, allowed spl tokens, programs config

> Note: only `signAndSend` submits a transaction to an RPC, all other methods only return a signed transaction

## Quick Start

Set up a local Kora server and start accepting SPL token payments for transaction fees in just a few minutes.

**[→ Quick Start Guide](docs/getting-started/QUICK_START.md)**

## Crates

- `kora-lib`: Shared library for kora
- `kora-rpc`: JSON-RPC server
- `kora-cli`: Command line interface for kora
- `turnkey`: Turnkey signer library
- `privy`: Privy signer library

## Getting Started

### Installation

```bash
git clone https://github.com/solana-foundation/kora.git
cd kora
make install
```

### Running the Server

Basic usage:

```bash
kora-rpc -- [OPTIONS]
```

### Configuration

#### Command Line Arguments

| Option                                                | Description                            | Default               |
| ----------------------------------------------------- | -------------------------------------- | --------------------- |
| `-p, --port <PORT>`                                   | Port number for the RPC server         | 8080                  |
| `--rpc-url <RPC_URL>`                                 | RPC URL to connect to                  | http://127.0.0.1:8899 |
| `--logging-format <FORMAT>`                           | Logging format (standard or json)      | standard              |
| `--metrics-endpoint <ENDPOINT>`                       | Optional metrics endpoint URL          | -                     |
| `--private-key <PRIVATE_KEY>`                         | Base58-encoded private key for signing | -                     |
| `--config <FILE>`                                     | Path to kora.toml config file          | kora.toml             |

#### Signer Configuration

**General**

| Option                                                | Description                            | Default               |
| ----------------------------------------------------- | -------------------------------------- | --------------------- |
| `--no-load-signer`                                    | Skip loading the signer                | false                 |

**Turnkey**

| Option                                                | Description                            | Default               |
| ----------------------------------------------------- | -------------------------------------- | --------------------- |
| `--with-turnkey-signer`                               | Use Turnkey signer                     | false                 |
| `--turnkey-api-public-key <TURNKEY_API_PUBLIC_KEY>`   | Turnkey API public key                 | -                     |
| `--turnkey-api-private-key <TURNKEY_API_PRIVATE_KEY>` | Turnkey API private key                | -                     |
| `--turnkey-organization-id <TURNKEY_ORGANIZATION_ID>` | Turnkey organization ID                | -                     |
| `--turnkey-private-key-id <TURNKEY_PRIVATE_KEY_ID>`   | Turnkey private key ID                 | -                     |
| `--turnkey-public-key <TURNKEY_PUBLIC_KEY>`           | Turnkey public key                     | -                     |

**Privy**

| Option                                                | Description                            | Default               |
| ----------------------------------------------------- | -------------------------------------- | --------------------- |
| `--with-privy-signer`                                 | Use Privy signer                       | false                 |
| `--privy-app-id <PRIVY_APP_ID>`                       | Privy App ID                           | -                     |
| `--privy-app-secret <PRIVY_APP_SECRET>`               | Privy App Secret                       | -                     |
| `--privy-wallet-id <PRIVY_WALLET_ID>`                 | Privy Wallet ID                       | -                     |


#### Environment Variables

| Variable                  | Description                                        | Example           |
| ------------------------- | -------------------------------------------------- | ----------------- |
| `RUST_LOG`                | Controls log level and filtering                   | "info,sqlx=error" |
| `RPC_URL`                 | Alternative way to specify the RPC URL             | -                 |
| `KORA_PRIVATE_KEY`        | Alternative way to specify the signing private key | -                 |
| `TEST_SENDER_KEYPAIR `    | Test sender base 58 private key                    | -                 |

#### Signer Environment Variables

**Turnkey Environment Variables**

| Variable                  | Description                                        | Example           |
| ------------------------- | -------------------------------------------------- | ----------------- |
| `TURNKEY_API_PUBLIC_KEY`  | Turnkey API public key                             | -                 |
| `TURNKEY_API_PRIVATE_KEY` | Turnkey API private key                            | -                 |
| `TURNKEY_ORGANIZATION_ID` | Turnkey organization ID                            | -                 |
| `TURNKEY_PRIVATE_KEY_ID`  | Turnkey private key ID                             | -                 |
| `TURNKEY_PUBLIC_KEY`      | Turnkey public key                                 | -                 |
| `TURNKEY_API_KEY`         | Turnkey API key                                    | -                 |

**Privy Environment Variables**

| Variable                  | Description                                        | Example           |
| ------------------------- | -------------------------------------------------- | ----------------- |
| `PRIVY_APP_ID`            | Privy app ID                                       | -                 |
| `PRIVY_APP_SECRET`           | Privy API key                                      | -                 |
| `PRIVY_WALLET_ID  `       | Privy wallet ID                                    | -                 |


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

[validation.fee_payer_policy]
allow_sol_transfers = true
allow_spl_transfers = true
allow_token2022_transfers = true
allow_assign = true
```

### Fee Payer Policy

The fee payer policy system allows you to control what actions the fee payer can perform in transactions. This provides enhanced security and flexibility.

#### Configuration Options

- `allow_sol_transfers`: Allow the fee payer to be the source account in SOL transfers
- `allow_spl_transfers`: Allow the fee payer to be the source/signer in SPL token transfers
- `allow_token2022_transfers`: Allow the fee payer to be the source/signer in Token2022 transfers
- `allow_assign`: Allow the fee payer to use the Assign instruction (change account owner)

#### Example Configurations

**Default (Permissive - Backward Compatible)**:
```toml
[validation.fee_payer_policy]
allow_sol_transfers = true
allow_spl_transfers = true
allow_token2022_transfers = true
allow_assign = true
```

**Restrictive (Enhanced Security)**:
```toml
[validation.fee_payer_policy]
allow_sol_transfers = false
allow_spl_transfers = false
allow_token2022_transfers = false
allow_assign = false
```

**Selective (Only SOL and Token2022)**:
```toml
[validation.fee_payer_policy]
allow_sol_transfers = true
allow_spl_transfers = false
allow_token2022_transfers = true
allow_assign = false
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
            "transaction": "<base64-encoded-transaction>",
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
            "transaction": "<base64-encoded-transaction>"
        }
    ]
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<base64-encoded-signed-transaction>"
    }
}
```

#### `signAndSendTransaction`

Signs and submits a transaction to the network.

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "signAndSendTransaction",
    "params":  {
        "transaction": "<base64-encoded-transaction>"
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<bas64-encoded-signed-transaction>"
    }
}
```

#### `transferTransaction`

Create a transfer request and sign as the paymaster (SPL and SOL)

```json
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "transferTransaction",
    "params": {
        "amount": 1000000,
        "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "source": "5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht",
        "destination": "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM"
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "transaction": "<base64-encoded-transaction>",
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
        "transaction": "<base64-encoded-transaction>"
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
    "params":  {
        "transaction": "<base64-encoded-transaction>",
        "margin": 0.0
    }
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "signature": "<base58-encoded-signature>",
        "signed_transaction": "<base64-encoded-signed-transaction>"
    }
}
```

## Development

### Prerequisites

- Rust 1.70 or later
- Solana CLI tools (for testing)
- A Solana RPC endpoint (local or remote)
- Docker
- swagger-cli (`npm install -g swagger-cli`) for API documentation

### Building

```bash
# Build
make build
```

### Testing

```bash
# Run all tests
make test

# Setup test environment (for integration tests)
make setup-test-env

# Run integration tests
make test-integration
```

#### Integration Test Environment Setup

Integration tests require additional setup for test accounts and local validator:

1. **Start local validator:**
   ```bash
   solana-test-validator --reset
   ```

2. **Start local Kora Server:**
    ```bash
    make run
    ```

3. **Run integration tests:**
   ```bash
   make test-integration
   ```
    This will initialize a test environment (cargo run -p tests --bin setup-test-env):
   - Verify test validator is running
   - Create and fund test accounts
   - Set up USDC mint and token accounts
   - Display account summary

   And run all integration tests (cargo test --test integration)

#### Customize Test Environment

You can customize test behavior by setting environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `RPC_URL` | Solana RPC endpoint | `http://127.0.0.1:8899` |
| `TEST_SERVER_URL` | Kora RPC server URL | `http://127.0.0.1:8080` |
| `TEST_SENDER_KEYPAIR` | Base58 encoded test sender keypair | Built-in test keypair |
| `TEST_RECIPIENT_PUBKEY` | Test recipient public key | Built-in test pubkey |
| `KORA_PRIVATE_KEY` | Kora fee payer private key | Built-in test keypair |
| `TEST_USDC_MINT_KEYPAIR` | Test USDC mint keypair | Built-in test mint |
| `TEST_USDC_MINT_DECIMALS` | USDC mint decimals | `6` |

Make sure to update kora.toml to reflect the public key of TEST_USDC_MINT_KEYPAIR.

**Example with custom environment:**
```bash
# Create .env file
echo "RPC_URL=https://api.devnet.solana.com" > .env
echo "TEST_SENDER_KEYPAIR=your_base58_keypair" >> .env

# Run tests
make test-integration
```

### Running

```bash
# Basic
kora  \
    --rpc-url <RPC_URL> \

# With Turnkey (or use environment variables)
kora  \
    --rpc-url <RPC_URL> \
    --with-turnkey-signer \
    --turnkey-api-public-key <TURNKEY_API_PUBLIC_KEY> \
    --turnkey-api-private-key <TURNKEY_API_PRIVATE_KEY> \
    --turnkey-organization-id <TURNKEY_ORGANIZATION_ID> \
    --turnkey-private-key-id <TURNKEY_PRIVATE_KEY_ID> \
    --turnkey-public-key <TURNKEY_PUBLIC_KEY>

# With Privy (or use environment variables)
kora \
    --rpc-url <RPC_URL> \
    --with-privy-signer \
    --privy-app-id <PRIVY_APP_ID> \
    --privy-app-secret <PRIVY_APP_SECRET> \
    --privy-wallet-id <PRIVY_WALLET_ID>

# No signer
kora  \
    --rpc-url <RPC_URL> \
    --port <PORT> \
    --no-load-signer

# Load private key at runtime without .env
kora  \
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

# Openapi
make openapi
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
