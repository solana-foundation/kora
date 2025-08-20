# Kora Node Operator Guide

## What is a Kora Node Operator?

As a Kora node operator, you run a **paymaster service** that sponsors Solana transaction fees for your users. Your node accepts SPL token payments and uses your SOL to pay actual network fees, enabling gasless transactions for your application.

## Why Run a Kora Node?

- **Better UX**: Your users transact without needing SOL (streamlined onboarding, better retention, etc.)
- **Revenue Stream**: Collect fees in tokens your business prefers (USDC, BONK, etc.)

## Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Kora CLI](#kora-cli)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Need Help?](#need-help)

## Quick Start

**Want to try locally first? Spin up a local Kora node in a few minutes:** → **[Local Quick Start Guide](../getting-started/QUICK_START.md)**

## Core Concepts

As a Kora node operator, you're responsible for running a secure paymaster service that enables gasless transactions for your users. Your node has four key responsibilities:

### 1. Validate Transactions
Configure your node to accept only transactions that meet your business requirements via `kora.toml`:
- **Token allowlists**: Define which SPL tokens you accept as payment (supports both SPL and Token-2022)
- **Program allowlists**: Whitelist which Solana programs users can interact with
- **Transaction limits**: Set maximum fees you're willing to pay and signature limits
- **Account blocklists**: Prevent interactions with problematic addresses
- **Pricing oracles**: Configure Jupiter or mock pricing for fee calculations
- **Payment tokens**: Specify which type of tokens you will accept as payment 
- **Feepayer policies**: Control what operations your feepayer can perform (transfers, burns, approvals, etc.)
- **Token-2022 extensions**: Block specific Token-2022 extensions for enhanced security
- **Caching**: Enable Redis caching to improve performance by reducing RPC calls

**[→ Complete Kora.toml Configuration Reference](CONFIGURATION.md)**
**[→ Sample kora.toml](./deploy/sample/kora.toml)**

### 2. Sign Transactions  
Your node needs a Solana keypair to sign transactions as the fee payer. Consider implementing key rotation, access controls, backups, and other strategies for signer security. At present, Kora enables you to sign with:
- **Environment Variables**: Store private key directly in server environment (as base58, .json file, or u8 array)
- [**Turnkey**](https://www.turnkey.com/): Turnkey is private key management made simple. Create wallets, sign transactions, and automate onchain actions — all with one elegant API.
- [**Privy**](https://www.privy.io/): Privy makes it easy to build on crypto rails. Securely spin up whitelabel wallets, sign transactions, and integrate onchain infrastructure—all through one simple API.

### 3. Pay Transaction Fees
Maintain sufficient SOL to cover network fees for your expected transaction volume:
- **Balance monitoring**: Track SOL balance and set up low-balance alerts
- **Automation**: Implement automatic SOL top-up procedures for production environments
- **Capacity planning**: Plan for expected use case, user volume, etc.

### 4. Monitor Operations
Continuously track your node's security, performance, and business metrics:
- **Security monitoring**: Unusual patterns, failed validations, and rate limit breaches
- **Operational alerts**: System health, balance warnings, and security events
- **Financial tracking**: SOL costs vs. token revenue, profitability analysis

Kora provides an optional `/metrics` endpoint that provides real-time performance data in Prometheus format.

**[→ Kora Monitoring Reference Guide](./MONITORING.md)**

### 5. Optimize Performance (Optional)
For high-traffic deployments, enable Redis caching to reduce RPC calls and improve response times:
- **Account caching**: Cache Solana account data with configurable time to live (TTL)
- **Automatic fallback**: Gracefully falls back to direct RPC calls if Redis is unavailable
- **Cache management**: Automatic expiration and force-refresh capabilities for critical operations

## Kora CLI

The [kora-cli crate](https://crates.io/crates/kora-cli) is a production-ready Rust binary that provides everything you need to run a Kora paymaster node. It's distributed as a standalone executable that you can install globally or deploy in containers.

### Installation Options

**Global Installation (recommended for development)**
```bash
cargo install kora-cli
```

**Or build from source  (recommended for contributing)**

```bash
git clone https://github.com/solana-foundation/kora.git
cd kora
make install
```

### Basic Usage

The Kora RPC server exposes a JSON-RPC endpoint (default: http://localhost:8080). Launch it with the `kora rpc` command:

```bash
# Run with default configuration (looks for .env and kora.toml in current directory)
kora rpc

# Specify custom configuration path
kora --config /path/to/kora.toml rpc

# Specify private key location
kora rpc --private-key /path/to/keypair.json

# Set custom port
kora rpc --port 3000

# Help
kora rpc --help
```

Applications can access the Kora RPC Server via the [Kora CLI](../../crates/cli/) or the [Kora TS SDK](../../sdks/ts/)

## Configuration 

Every Kora RPC node must be configured with at least:
- a Solana RPC endpoint (specified via the `--rpc-url` flag or `RPC_URL` environment variable) [default: http://127.0.0.1:8899]
- a Solana signer (specified via the `--private-key` or `KORA_PRIVATE_KEY=` environment variable) (Check out the **[Signers Documentation](./SIGNERS.md)** for signing with a key management provider)
- a config file, `kora.toml` (specified via the `--config path/to/kora.toml` flag)

**kora.toml**

Before deploying, you'll need to create and configure a `kora.toml` to specify:

- Rate limiting and authentication
- RPC method availability
- Transaction validation rules
- Security policies (whitelist or blacklist of SPL tokens, programs, accounts, token extensions, etc.)
- Fee pricing models
- Enhanced fee payer policies (protect against unwanted signer behavior)
- Metrics collection
- Redis caching configuration (optional)

## Deployment 

### Local Deployment

Start up and test a local Kora Server in minutes: [Quick Start Guide](../getting-started/QUICK_START.md)

### Docker

Use the sample Dockerfile to deploy on any container platform. The docker-compose.yml file includes Redis for caching support:

**[→ Sample Dockerfile](./deploy/sample/Dockerfile)**
**[→ Docker Compose with Redis](../../docker-compose.yml)**

### Platform-Specific Guides

- **[Railway Deployment](deploy/RAILWAY.md)** 

*More integration guides coming soon*


## Need Help?

- **[Solana Stack Exchange](https://solana.stackexchange.com/)** - Ask questions/share learnings (make sure to use the `kora` tag)
- **[GitHub Issues](https://github.com/solana-foundation/kora/issues)** - Report bugs or get help
- Run `kora-rpc --help` to see all available flags and configuration options




