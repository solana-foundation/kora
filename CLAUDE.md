# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## TL;DR - Development Workflow

### Branches & Commits
- **Main branch**: `main` (protected, requires PRs)
- **Release branch**: `release/{id}` (requires PRs)
- **Commit format**: Use conventional commits for automatic releases
  - `feat:` → minor version bump (1.0.3 → 1.1.0)
  - `fix:` → patch version bump (1.0.3 → 1.0.4)
  - `BREAKING CHANGE:` → major version bump (1.0.3 → 2.0.0)
  - `chore:`, `docs:`, `refactor:` → patch version bump

### Publishing Flow
- **Rust crates**: Automatic publishing when files change + conventional commits
- **TypeScript SDKs**: Changeset-based releases (require `pnpm changeset`)
- **Individual crate releases**: Each crate publishes independently when modified
- **GitHub releases**: Auto-generated with commit-based release notes

## Project Overview

Kora is a Solana paymaster node that provides a JSON-RPC interface for handling gasless transactions and fee abstractions. It enables developers to build applications where users can pay transaction fees in tokens other than SOL.

The repository consists of 2 main workspace crates:

- `kora-lib`: Core library with integrated RPC server functionality, signers, transaction handling, and configuration
- `kora-cli`: Unified command-line interface with RPC server and configuration commands
- `tests`: Integration tests for the entire workspace
- `sdks/`: TypeScript SDKs for client integration

## TL;DR - Authentication Methods

Kora supports two authentication methods that can be used individually or together:

1. **API Key Authentication**: Simple header-based auth using `x-api-key` header
2. **HMAC Authentication**: Request signature auth using `x-timestamp` and `x-hmac-signature` headers

**Testing:**
```bash
make test-integration           # Run all integration tests
```

## Common Development Commands

### Build & Check

```bash
# Build all workspace packages
make build

# Build specific packages
make build-lib    # Build the lib crate
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

# Setup test environment (for integration tests)
make setup-test-env

# Run integration tests
make test-integration

# Run all tests
cargo test --workspace
```

#### Integration Test Environment Setup

Integration tests are fully automated using a makefile-based approach that handles sequential test execution:

**Quick Start:**
```bash
make test-integration
```

**What happens automatically:**
1. **Solana Validator**: Starts local test validator with reset
2. **Test Environment Setup**: Creates test accounts, tokens, and ATAs
3. **Sequential Test Phases**: Runs 3 test suites with different configurations

**Test Phases (Auto-managed):**

**Phase 1: Regular integration tests (26 tests)**
- Config: `tests/src/common/fixtures/kora-test.toml` (no auth)
- Server: Standard Kora RPC server 
- Tests: Core RPC functionality, token operations, compute budget

**Phase 2: Auth integration tests (4 tests)**  
- Config: `tests/src/common/fixtures/auth-test.toml` (auth enabled)
- Server: Restart with auth middleware
- Tests: API key and HMAC authentication validation

**Phase 3: Payment address tests (2 tests)**
- Config: `tests/src/common/fixtures/paymaster-address-test.toml` (payment address)
- Server: Restart with payment address configuration
- **CLI ATA Initialization**: Automatically runs `kora rpc initialize-atas` before tests
- Tests: Payment address validation and wrong destination rejection

**File Structure:**
```
tests/
├── src/
│   ├── common/
│   │   ├── fixtures/
│   │   │   ├── kora-test.toml           # Regular tests config
│   │   │   ├── auth-test.toml           # Auth tests config  
│   │   │   └── paymaster-address-test.toml  # Payment address config
│   │   ├── local-keys/
│   │   │   ├── fee-payer-local.json     # Fee payer keypair
│   │   │   ├── payment-local.json       # Payment address keypair
│   │   │   ├── sender-local.json        # Sender keypair
│   │   │   └── usdc-mint-local.json     # USDC mint keypair
│   │   └── setup_test_env.rs            # Test environment setup
│   └── bin/
│       └── setup_test_env.rs            # Binary wrapper for setup
├── integration/                         # Regular integration tests
├── auth/                                # Authentication tests
└── payment-address/                     # Payment address tests
```

**Manual Test Commands (if needed):**
```bash
# Setup test environment only
make setup-test-env

# Clean up test environment
make clean-test-env

# Run specific test phase
cargo test -p tests --test integration
cargo test -p tests --test auth  
cargo test -p tests --test payment-address
```

#### Customize Test Environment

The test suite uses environment variables for configuration (checked before falling back to defaults):

| Variable | Description | Default |
|----------|-------------|---------|
| `RPC_URL` | Solana RPC endpoint | `http://127.0.0.1:8899` |
| `TEST_SERVER_URL` | Kora RPC server URL | `http://127.0.0.1:8080` |
| `TEST_SENDER_KEYPAIR` | Base58 encoded test sender keypair | Built-in test keypair |
| `TEST_RECIPIENT_PUBKEY` | Test recipient public key | Built-in test pubkey |
| `KORA_PRIVATE_KEY` | Kora fee payer private key | Built-in test keypair |
| `TEST_USDC_MINT_KEYPAIR` | Test USDC mint keypair | Built-in test mint |
| `TEST_USDC_MINT_DECIMALS` | USDC mint decimals | `6` |

Make sure to update the appropriate config file (kora.toml for production, tests/common/fixtures/kora-test.toml for testing) to reflect the public key of TEST_USDC_MINT_KEYPAIR.

**Example with custom test configuration:**
```bash
# Create .env file for custom test setup
echo "RPC_URL=https://api.devnet.solana.com" > .env
echo "TEST_SENDER_KEYPAIR=your_base58_keypair" >> .env
echo "KORA_PRIVATE_KEY=your_fee_payer_keypair" >> .env

# Run tests with custom config
make test-integration
```

### Running Services

```bash
# Basic server run (production config)
make run

# Run with test configuration (for integration testing)
cargo run -p kora --bin kora -- --config tests/common/fixtures/kora-test.toml --rpc-url http://127.0.0.1:8899 rpc --signers-config tests/common/fixtures/signers.toml

# Run with debug logging
RUST_LOG=debug cargo run -p kora --bin kora -- rpc start

# Run RPC server with all parameters
cargo run -p kora --bin kora -- --config kora.toml --rpc-url https://api.devnet.solana.com rpc start \
  --port 8080 \
  --logging-format standard

# Run with Turnkey signer
cargo run -p kora --bin kora -- rpc start --signers-config path/to/turnkey-signers.toml

# Run with Privy signer
cargo run -p kora --bin kora -- rpc start --signers-config path/to/privy-signers.toml

# Run with Vault signer  
cargo run -p kora --bin kora -- rpc start \
  --signers-config path/to/vault-signers.toml

# Configuration validation commands
cargo run -p kora --bin kora -- config validate
cargo run -p kora --bin kora -- config validate-with-rpc

# Generate OpenAPI documentation
cargo run -p kora --bin kora --features docs -- openapi -o openapi.json
```

### TypeScript SDK Development

```bash
# In sdks/ts/
pnpm run build
pnpm run test
pnpm run lint
pnpm run format
```

## Architecture Overview

### Core Library (`kora-lib/src/`)

- **signer/** - Abstraction layer supporting multiple signer types (configured in `signers.toml`)
  - `SolanaMemorySigner` - Local keypair signing
  - `VaultSigner` - HashiCorp Vault integration
  - `TurnkeySigner` - Turnkey API integration  
  - `PrivySigner` - Privy API integration
  - Unified `KoraSigner` enum with trait implementation
  - optionally, `--no-signer` flag to run Kora without a signer

- **transaction/** - Transaction processing pipeline:
  - Fee estimation and calculation
  - Transaction validation against configuration rules
  - Paid transaction verification
  - Solana transaction utilities
  - **Lookup Table Resolution**: V0 transactions require address lookup table resolution for proper fee calculation and validation. The system uses `VersionedTransactionExt` trait and `VersionedTransactionResolved` wrapper for efficient caching of resolved addresses.

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

### RPC Server (now in `kora-lib/src/rpc_server/`)

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
  - `getPayerSigner` - Get the payer signer and payment destination
  - (client-only) `getPaymentInstruction` - Get a payment instruction for a transaction

- **openapi/** - Auto-generated API documentation using `utoipa`
- **args.rs** - RPC-specific command line arguments

### CLI Tool (`kora-cli/src/`)

- Unified command-line interface with subcommands:
  - `kora config validate` - Validate configuration file
  - `kora config validate-with-rpc` - Validate configuration with RPC calls
  - `kora rpc start --signers-config path/to/signers.toml` - Start the RPC server with all signer options (kora.toml in cwd)
  - `kora --config path/to/kora.toml rpc start --signers-config path/to/signers.toml` - Start the RPC server with specific config and signers
  - `kora openapi` - Generate OpenAPI documentation
- Global arguments (rpc-url, config) separated from RPC-specific arguments
- All signer types supported for RPC server mode

**Example CLI usage:**
```bash
# Validate configuration
cargo run -p kora --bin kora -- --config kora.toml config validate

# Start RPC server with local private key
cargo run -p kora --bin kora -- --config path/to/kora.toml --rpc-url https://api.devnet.solana.com rpc start --signers-config path/to/signers.toml

# Start RPC server with Turnkey signer
cargo run -p kora --bin kora -- rpc start --signers-config path/to/turnkey-signers.toml

# Generate OpenAPI documentation
cargo run -p kora --bin kora --features docs -- openapi -o openapi.json
```

### Signer Integrations

- **kora-turnkey** - Turnkey key management API integration (separate crate)
- **kora-privy** - Privy wallet API integration (separate crate)  
- **VaultSigner** - HashiCorp Vault integration (built into kora-lib)
- Remote signers integrate via HTTP APIs to external services

### TypeScript SDKs

- **sdks/ts/** - Main TypeScript SDK for client integration
- Provide typed interfaces for all RPC methods

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

# Fee payer policy controls what actions the fee payer can perform
# All default to true for backward compatibility
[validation.fee_payer_policy]
allow_sol_transfers = true      # Allow fee payer to be source in SOL transfers
allow_spl_transfers = true      # Allow fee payer to be source in SPL token transfers
allow_token2022_transfers = true # Allow fee payer to be source in Token2022 transfers
allow_assign = true             # Allow fee payer to use Assign instruction
```

### Environment Variables set in `signers.toml`

**General:**
```bash
RUST_LOG=debug  # Logging level
```

## Fee Payer Policy System

### Overview

The fee payer policy system provides fine-grained control over what actions the fee payer can perform in transactions. By default, all actions are permitted to maintain backward compatibility with existing behavior.

### Policy Configuration

The fee payer policy is configured via the `[validation.fee_payer_policy]` section in `kora.toml`:

### Implementation Details

**Core Structure** (`crates/lib/src/config.rs`):
- `FeePayerPolicy` struct with 4 boolean fields
- `Default` implementation sets all fields to `true` (permissive)
- `#[serde(default)]` attribute ensures backward compatibility

**Validation Logic** (`crates/lib/src/transaction/validator.rs`):
- `TransactionValidator` stores the policy configuration
- `is_fee_payer_source()` method checks policy flags before validating restrictions
- Different validation logic for each program type (System, SPL Token, Token2022)

**Supported Actions**:
1. **SOL Transfers** - System program Transfer and TransferWithSeed instructions
2. **SPL Token Transfers** - SPL Token program Transfer and TransferChecked instructions
3. **Token2022 Transfers** - Token2022 program Transfer and TransferChecked instructions
4. **Assign** - System program Assign instruction (changes account owner)

## Private Key Formats

Kora supports multiple private key formats for enhanced usability and compatibility with different tooling, each specified in `signers.toml`

### 1. Base58 Format
Traditional Solana private key format - base58-encoded 64-byte private key:
```bash
KORA_PRIVATE_KEY=your_base58_private_key
```

### 2. U8Array Format
Comma-separated byte array format compatible with Solana CLI outputs:
```bash
KORA_PRIVATE_KEY="[123,45,67,89,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56,78,90,12,34,56]"
```

### 3. JSON File Path
Path to a JSON file containing the private key:
```bash
KORA_PRIVATE_KEY="/path/to/keypair.json"
```

### Format Detection
The system automatically detects the format based on input patterns:
1. **File path** - Attempts to read as file first
2. **U8Array** - Detects `[...]` format
3. **Base58** - Default fallback format

### Environment Variables
All private key environment variables support the same multiple formats.

## Lookup Table Resolution System

### Overview

Kora handles both legacy and V0 versioned transactions. V0 transactions use address lookup tables to compress transaction size by referencing frequently used addresses. Before processing V0 transactions for fee calculation or validation, these lookup tables must be resolved to get the actual addresses.

### Design Pattern

**Trait-Based Abstraction**: The system uses the `VersionedTransactionExt` trait to provide a unified interface for both legacy and V0 transactions:

```rust
pub trait VersionedTransactionExt {
    fn get_all_account_keys(&self) -> Vec<Pubkey>;
    fn get_transaction(&self) -> &VersionedTransaction;
}
```

**Caller Responsibility Pattern**: Following Rust best practices, the system uses caller responsibility where expensive operations (RPC calls to resolve lookup tables) are explicit:

- `VersionedTransaction` - Implements trait directly, returns only static account keys
- `VersionedTransactionResolved<'a>` - Wrapper that caches resolved addresses after calling `resolve_addresses(rpc_client).await`


## Transaction Flow

1. **Client Request** - Client submits transaction to RPC endpoint
2. **Validation** - Transaction validated against configuration rules including fee payer policy
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

## Authentication Methods

Kora supports two optional authentication methods:

1. **API Key Auth** (`api_key` in kora.toml): Simple header-based auth using `x-api-key`
2. **HMAC Auth** (`hmac_secret` in kora.toml): Secure signature-based auth using `x-timestamp` + `x-hmac-signature` (SHA256 of timestamp+body)

Both skip `/liveness` endpoint and can be used simultaneously. Implementation uses async tower middleware for non-blocking concurrent requests.

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

## Makefile Structure

The project uses a modular makefile approach for better organization:

```
/Makefile                  # Main makefile with includes
/makefiles/
├── utils.mk              # Common utilities and functions
├── rs.mk                 # Rust build commands  
├── tests_rs.mk           # Integration test orchestration
└── docs.mk               # Documentation generation
```

**Key Features:**
- **Sequential Test Execution**: Prevents port conflicts and ensures clean config isolation
- **Server Lifecycle Management**: Automatic start/stop with proper cleanup
- **CLI Integration**: Automated `kora rpc initialize-atas` for payment address tests
- **Parallel Tool Execution**: Optimized for development workflow efficiency

**Core Functions (from utils.mk):**
- `start_solana_validator` / `stop_solana_validator`: Validator lifecycle
- `start_kora_server` / `stop_kora_server`: Server lifecycle with config switching
- `run_integration_phase`: Orchestrates server + CLI + tests for each phase

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

## Key Integration Test Achievements

**✅ Complete Sequential Test Runner**
- Makefile-based orchestration with 3 automated phases
- Proper server lifecycle management (start/stop/restart)
- Config isolation between test suites
- Zero manual intervention required

**✅ CLI Integration for Payment Address Tests**
- Automated `kora rpc initialize-atas` execution before payment tests
- Real-world workflow simulation (CLI → tests)
- Payment address ATA creation and validation

**✅ Payment Validation Testing**
- Tests payment address validation logic (not transaction simulation)
- Proper positive/negative test cases:
  - ✅ Transfer to payment address → should succeed  
  - ✅ Transfer to wrong destination → should fail validation
- Removed transaction simulation dependency (caused AccountNotFound issues)

**✅ Test Infrastructure**
- File reorganization: `tests/src/common/` structure for proper Rust crate organization
- Config files: `kora-test.toml`, `auth-test.toml`, `paymaster-address-test.toml`
- Binary setup: `setup_test_env` binary for account/token initialization
- Path resolution fixes for workspace vs. crate directory differences

**Test Results:**
- Integration tests: **26/26 passed** ✅
- Auth tests: **4/4 passed** ✅  
- Payment address tests: **2/2 passed** ✅
- CLI ATA initialization: **Working** ✅
