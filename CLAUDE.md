# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kora is a Solana paymaster node that provides a JSON-RPC interface for handling gasless transactions and fee abstractions. It enables developers to build applications where users can pay transaction fees in tokens other than SOL.

The repository consists of 5 main workspace crates:

- `kora-lib`: Core library containing shared functionality, signers, transaction handling, and configuration
- `kora-rpc`: JSON-RPC server implementation with middleware support
- `kora-cli`: Command-line interface for transaction operations
- `kora-turnkey`: Turnkey signer integration library
- `kora-privy`: Privy signer integration library
- `tests`: Integration tests for the entire workspace
- `scripts`: Utility scripts for development
- `sdks/`: TypeScript SDKs for client integration

## Common Development Commands

### Build & Check

```bash
# Build all workspace packages
make build

# Build specific packages
make build-lib    # Build the lib crate
make build-rpc    # Build the RPC server
make build-cli    # Build the CLI tool

# Install all binaries
make install

# Check formatting
make check

# Format code
make fmt

# Run linter with warnings as errors
make lint

# Run linter with auto-fix
make lint-fix-all
```

### Testing

```bash
# Run unit tests
make test

# Run integration tests
make test-integration

# Run all tests
cargo test --workspace
```

### Running Services

```bash
# Basic server run
make run

# Run with debug logging
RUST_LOG=debug cargo run -p kora-rpc --bin kora-rpc

# Run RPC server with all parameters
cargo run -p kora-rpc --bin kora-rpc -- \
  --port 8080 \
  --config kora.toml \
  --rpc-url https://api.devnet.solana.com \
  --logging-format standard \
  --metrics-endpoint http://localhost:9090

# Run with Turnkey signer
cargo run -p kora-rpc --bin kora-rpc -- \
  --with-turnkey-signer \
  --turnkey-api-public-key $TURNKEY_API_PUBLIC_KEY \
  --turnkey-api-private-key $TURNKEY_API_PRIVATE_KEY \
  --turnkey-organization-id $TURNKEY_ORGANIZATION_ID \
  --turnkey-private-key-id $TURNKEY_PRIVATE_KEY_ID \
  --turnkey-public-key $TURNKEY_PUBLIC_KEY \
  --port 8080

# Run with Privy signer
cargo run -p kora-rpc --bin kora-rpc -- \
  --with-privy-signer \
  --privy-app-id $PRIVY_APP_ID \
  --privy-app-secret $PRIVY_APP_SECRET \
  --privy-wallet-id $PRIVY_WALLET_ID

# Run with Vault signer  
cargo run -p kora-rpc --bin kora-rpc -- \
  --vault-signer \
  --vault-addr $VAULT_ADDR \
  --vault-token $VAULT_TOKEN \
  --vault-key-name $VAULT_KEY_NAME \
  --vault-pubkey $VAULT_PUBKEY

# Run CLI commands
cargo run -p kora-cli --bin kora-cli -- [subcommand]
```

### TypeScript SDK Development

```bash
# In sdks/ts/
pnpm run build
pnpm run test
pnpm run lint
pnpm run format

# In sdks/net-ts/
pnpm run build
pnpm run test
pnpm run lint
pnpm run format
```

## Architecture Overview

### Core Library (`kora-lib/src/`)

- **signer/** - Abstraction layer supporting multiple signer types:
  - `SolanaMemorySigner` - Local keypair signing
  - `VaultSigner` - HashiCorp Vault integration
  - `TurnkeySigner` - Turnkey API integration  
  - `PrivySigner` - Privy API integration
  - Unified `KoraSigner` enum with trait implementation

- **transaction/** - Transaction processing pipeline:
  - Fee estimation and calculation
  - Transaction validation against configuration rules
  - Paid transaction verification
  - Solana transaction utilities

- **token/** - SPL token handling:
  - Token interface abstractions (SPL vs Token-2022)
  - Token account management
  - Token validation and metadata

- **oracle/** - Price feed integration:
  - Jupiter API integration for token pricing
  - Price calculation for fee estimation

- **config.rs** - TOML-based configuration system with validation
- **state.rs** - Global signer state management
- **cache.rs** - Token account caching
- **rpc.rs** - Solana RPC client utilities

### RPC Server (`kora-rpc/src/`)

- **server.rs** - HTTP JSON-RPC server setup with middleware:
  - CORS configuration
  - Rate limiting
  - Proxy layer for health checks
  - Uses `jsonrpsee` for JSON-RPC protocol

- **method/** - RPC method implementations:
  - `estimateTransactionFee` - Calculate gas fees in different tokens
  - `signTransaction` - Sign transaction without broadcasting
  - `signAndSendTransaction` - Sign and broadcast to network
  - `transferTransaction` - Handle token transfers
  - `getBlockhash` - Get recent blockhash
  - `getConfig` - Return server configuration
  - `getSupportedTokens` - List accepted payment tokens
  - `signTransactionIfPaid` - Conditional signing based on payment verification

- **openapi/** - Auto-generated API documentation using `utoipa`

### CLI Tool (`kora-cli/src/`)

- Command-line interface with commands: `sign`, `sign-and-send`, `estimate-fee`, `sign-if-paid`
- Supports same configuration system as RPC server
- Can use any supported signer type

**Example CLI usage:**
```bash
# Sign transaction with local private key
cargo run -p kora-cli --bin kora-cli -- sign \
  --private-key your_base58_private_key \
  --config kora.toml \
  --rpc-url https://api.devnet.solana.com

# Sign with Turnkey
cargo run -p kora-cli --bin kora-cli -- sign \
  --with-turnkey-signer \
  --turnkey-api-public-key $TURNKEY_API_PUBLIC_KEY \
  --turnkey-api-private-key $TURNKEY_API_PRIVATE_KEY \
  --turnkey-organization-id $TURNKEY_ORGANIZATION_ID \
  --turnkey-private-key-id $TURNKEY_PRIVATE_KEY_ID \
  --turnkey-public-key $TURNKEY_PUBLIC_KEY
```

### Signer Integrations

- **kora-turnkey** - Turnkey key management API integration (separate crate)
- **kora-privy** - Privy wallet API integration (separate crate)  
- **VaultSigner** - HashiCorp Vault integration (built into kora-lib)
- Remote signers integrate via HTTP APIs to external services

### TypeScript SDKs

- **sdks/ts/** - Main TypeScript SDK for client integration
- **sdks/net-ts/** - Network-specific SDK with SAS (Solana Attestation Standard) utilities
- Provide typed interfaces for all RPC methods

**SAS Setup Scripts (`sdks/net-ts/scripts/`):**
- `setup-koranet-credential.ts` - Creates credentials for attestation
- `setup-koranet-example-attestation.ts` - Example attestation creation  
- `setup-koranet-schema.ts` - Schema setup for attestations
- Used for initializing SAS-based credential and attestation systems

## Configuration & Environment

### Main Configuration (`kora.toml`)

```toml
[kora]
rate_limit = 100  # Requests per second

[validation]
max_allowed_lamports = 1000000  # Maximum transaction value
max_signatures = 10             # Maximum signatures per transaction
price_source = "Mock"           # Price source: "Mock", "Jupiter", etc.

# Allowed Solana programs (by public key)
allowed_programs = [
    "11111111111111111111111111111111",      # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  # Token Program
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",  # Associated Token Program
]

# Supported tokens for fee payment (by mint address)
allowed_tokens = [
    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",  # USDC devnet
]

# SPL tokens accepted for paid transactions
allowed_spl_paid_tokens = [
    "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",  # USDC devnet
] 

disallowed_accounts = []  # Blocked account addresses
```

### Environment Variables

**Turnkey Integration:**
```bash
TURNKEY_API_PUBLIC_KEY=your_api_public_key
TURNKEY_API_PRIVATE_KEY=your_api_private_key  
TURNKEY_ORGANIZATION_ID=your_org_id
TURNKEY_PRIVATE_KEY_ID=your_private_key_id
TURNKEY_PUBLIC_KEY=your_public_key
```

**Privy Integration:**
```bash
PRIVY_APP_ID=your_app_id
PRIVY_APP_SECRET=your_app_secret
PRIVY_WALLET_ID=your_wallet_id
```

**HashiCorp Vault Integration:**
```bash
VAULT_ADDR=https://vault.example.com
VAULT_TOKEN=your_vault_token
VAULT_KEY_NAME=your_key_name
VAULT_PUBKEY=your_base58_public_key
```

**General:**
```bash
RUST_LOG=debug  # Logging level
```

## Transaction Flow

1. **Client Request** - Client submits transaction to RPC endpoint
2. **Validation** - Transaction validated against configuration rules
3. **Fee Calculation** - Fee calculated based on token type and current prices
4. **Signing** - Transaction signed using configured signer backend
5. **Response** - Signed transaction returned or broadcast to network

## Signer Architecture

- **Trait-based design** - All signers implement unified `Signer` trait
- **State management** - Global signer state with thread-safe access via `get_signer()`
- **Multiple backends** - Runtime selection between Memory, Vault, Turnkey, Privy
- **Initialization** - Lazy initialization with validation on first use
- **API Integration** - Turnkey and Privy use HTTP APIs for remote signing

## Code Style & Best Practices

### Async Development

- All RPC methods are async
- Use `tokio` runtime for async execution
- Signer operations are async to support remote API calls

## Code Quality

### Concurrency & Thread Safety

Kora is designed for high-performance concurrent operations:

- **Global State Management**: Use `Arc<Mutex<T>>` for shared state across threads
- **Signer State**: Global signer accessed via `get_signer()` with thread-safe initialization
- **RPC Server**: Handles multiple concurrent requests using `jsonrpsee` async framework
- **Cache Operations**: `TokenAccountCache` supports concurrent access for token account lookups
- **Token Account Access**: Always prioritize cache lookups before making on-chain RPC calls

### Async/Await Patterns

All I/O operations and external API calls are async:

- **RPC Client Operations**: Solana RPC calls are async to avoid blocking
- **Remote Signer APIs**: Turnkey and Privy API calls are async HTTP requests
- **Database Operations**: Token cache operations are async
- **Error Propagation**: Use `?` operator with async functions

### Logging Standards

Use structured logging throughout the codebase:

- **Error Level** (`log::error!`): System failures, critical errors, panics
- **Warn Level** (`log::warn!`): Recoverable errors, validation failures
- **Info Level** (`log::info!`): Important state changes, successful operations
- **Debug Level** (`log::debug!`): Detailed execution flow, parameter values
- **Trace Level** (`log::trace!`): Very verbose debugging information

**Logging Guidelines:**
- Include relevant context (transaction IDs, user addresses, amounts)
- Log entry and exit points for important operations
- Use structured data when possible for better parsing
- Never log sensitive information (private keys, secrets)
- Log errors with full context for debugging
- **CLI Output**: Use `println!` for CLI command results and user-facing output (not `log::info!`)

### Error Handling Patterns

- **Error Transformation**: Convert external errors to `KoraError` at module boundaries
- **Error Context**: Add meaningful context when propagating errors up the call stack
- **Error Classification**: Distinguish between recoverable validation errors and critical system failures
- **Error Responses**: Structure JSON-RPC error responses consistently across all methods

### Performance Guidelines

- **Memory Allocation**: Minimize allocations in hot paths, reuse buffers where possible
- **Connection Pooling**: Reuse HTTP clients and RPC connections across requests
- **Batch Operations**: Prefer batch APIs when available for multiple token account operations
- **Rate Limiting**: Implement client-side rate limiting for external API calls

### Security Practices

- **Secret Handling**: Never log, print, or serialize sensitive data (keys, tokens, secrets)
- **Input Sanitization**: Validate all user inputs against allow-lists and size limits
- **Audit Trail**: Log security-relevant events (authentication, authorization, signing)
- **Fail Secure**: Default to restrictive behavior when validation or authentication fails
- **Secure Communication**: Use secure communication for remote signer APIs
- **Rate Limiting & Authentication**: Implement proper rate limiting and authentication

### Testing Guidelines

- **Test Organization**: Mirror source code structure in test file organization
- **Mock Strategy**: Mock external dependencies (RPC clients, HTTP APIs) consistently
- **Test Data**: Use deterministic test data, avoid random values in tests
- **Integration Coverage**: Test complete request/response cycles for all RPC methods
- **Error Scenarios**: Test error conditions and edge cases, not just happy paths

### Testing Strategy

- **Unit tests** - Located in `src/` directories alongside source code
- **Integration tests** - Located in `tests/` directory for end-to-end workflows
- **API tests** - Include example JSON payloads in `tests/examples/`
- **SDK tests** - TypeScript tests in `sdks/*/test/` directories

## Development Guidelines

### Behavioral Instructions

- Always run linting and formatting commands before committing
- Use the Makefile targets for consistent builds across the workspace
- Test both unit and integration levels when making changes
- Verify TypeScript SDK compatibility when changing RPC interfaces

### Code Maintenance

- Follow existing patterns for RPC method implementations
- Add new signer types by implementing the `Signer` trait
- Update configuration schema when adding new validation rules
- Keep OpenAPI documentation in sync with method signatures
