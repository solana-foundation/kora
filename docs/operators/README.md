# Kora Node Operator Guide

## What is a Kora Node Operator?

As a Kora node operator, you run a **paymaster service** that sponsors Solana transaction fees for your users. Your node accepts SPL token payments and uses your SOL to pay actual network fees, enabling gasless transactions for your application.

## Why Run a Kora Node?

- **Better UX**: Your users transact without needing SOL (streamlined onboarding, better retention, etc.)
- **Revenue Stream**: Collect fees in tokens your business prefers (USDC, BONK, etc.)

## Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Kora-RPC Crate](#kora-rpc-crate)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Need Help?](#need-help)

## Quick Start

**Want to try locally first? Spin up a local Kora node in a few minutes:** → **[Local Quick Start Guide](../getting-started/QUICK_START.md)**

## Core Concepts

As a Kora node operator, you're responsible for running a secure paymaster service that enables gasless transactions for your users. Your node has four key responsibilities:

### 1. Validate Transactions
Configure your node to accept only transactions that meet your business requirements via `kora.toml`:
- **Token allowlists**: Define which SPL tokens you accept as payment
- **Program allowlists**: Whitelist which Solana programs users can interact with
- **Transaction limits**: Set maximum fees you're willing to pay and signature limits
- **Account blocklists**: Prevent interactions with problematic addresses
- **Pricing oracles**: Configure Jupiter or mock pricing for fee calculations
- **Payment tokens**: Specify which type of tokens you will accept as payment 
- **Feepayer policies**: Specify which types of instructions your feepayer can sign

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

## Kora-RPC Crate

The [kora-rpc crate](https://crates.io/crates/kora-rpc) is a production-ready Rust binary that provides everything you need to run a Kora paymaster node. It's distributed as a standalone executable that you can install globally or deploy in containers.

### Installation Options

**Global Installation (recommended for development)**
```bash
cargo install kora-rpc
```

**Or build from source  (recommended for contributing)**

```bash
git clone https://github.com/solana-foundation/kora.git
cd kora
make install
```

### Basic Usage

The Kora RPC server exposes a JSON-RPC endpoint (default: http://localhost:8080). Launch it with the `kora-rpc` command:

```bash
# Run with default configuration (looks for .env and kora.toml in current directory)
kora-rpc

# Specify custom configuration path
kora-rpc --config /path/to/kora.toml

# Specify private key location
kora-rpc --private-key /path/to/keypair.json

# Set custom port
kora-rpc --port 3000

# Help
kora-rpc --help
```

Applications can access the Kora RPC Server via the [Kora CLI](../../crates/cli/) or the [Kora TS SDK](../../sdks/ts/)

## Configuration 

Every Kora RPC node must be configured with at least:
- a Solana RPC endpoint (specified via the `--rpc-url` flag or `RPC_URL` environment variable) [default: http://127.0.0.1:8899]
- a Solana signer (specified via the `--private-key` or `KORA_PRIVATE_KEY=` environment variable) <!-- Check out our [Signers Dcoumentation](./SIGNERS.md) for signing with a key management provider -->
- a config file, `kora.toml` (specified via the `--config path/to/kora.toml` flag)

**kora.toml**

Before deploying, you'll need to create and configure a `kora.toml` to specify: payment tokens, security rules, rate limits, fee payer protections, and pricing oracle. Begin with tight limits, then gradually expand based on your application's needs.
Your `kora.toml` should live in the same directory as your deployment or be specified via the `--config` flag when starting the server.

<!-- **[→ Complete Kora.toml Configuration Reference](CONFIGURATION.md)** -->
**[→ Sample kora.toml](./deploy/sample/kora.toml)**



### 1. Rate Limiting

Implement global, per-second rate limiting for the server:

```toml
[kora]
rate_limit = 100
```

### 2. Payment Tokens

Specify the types of tokens you will accept as payment (e.g., USDC and Bonk):

```toml
[validation]
allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"   # BONK
]
```

### 3. Security Rules

Specify the maximum amount you are willing to pay, whitelist programs, or blacklist specific wallets

```toml
# [validation] (continued)
max_allowed_lamports = 1000000    # Max 0.001 SOL per transaction
max_signatures = 10               # Prevent complex transactions
allowed_programs = [              # Whitelist programs
    "11111111111111111111111111111111",  # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"  # Token Program
]
disallowed_accounts = []
```

### 4. Price Source (Oracle)

At present, you can choose 'Jupiter' for production applications or 'Mock' for testing. 

```toml
# [validation] (continued)
price_source = 'Jupiter'
```

### 5. Fee Payer Protections

Restrict your fee payer wallet from signing transactions that include certain types of instructions where the fee payer is the signer (e.g., Prevent transfers of SPL tokens that the fee payer is owner/authority of)

```toml
[validation.fee_payer_policy]
allow_sol_transfers = true      # Allow fee payer to be source in SOL transfers
allow_spl_transfers = true      # Allow fee payer to be source in SPL token transfers
allow_token2022_transfers = true # Allow fee payer to be source in Token2022 transfers
allow_assign = true             # Allow fee payer to use Assign instruction
```

### 6. User Authentication

Kora supports two optional authentication methods for securing your RPC endpoint: a global API key for all users or [HMAC authentication](https://en.wikipedia.org/wiki/HMAC) with replay protection. If neither is configured, no authentication is required to use the RPC endpoint. You can use both methods simultaneously for maximum security.

```toml
[kora]
# Simple API key authentication
api_key = "kora_live_sk_1234567890abcdef"

# Or HMAC authentication with replay protection  
hmac_secret = "your-strong-hmac-secret-minimum-32-chars"
```

<!-- **[→ Complete Kora.toml Configuration Reference](CONFIGURATION.md)** -->
For more information on authentication, see [Kora Authentication](./AUTHENTICATION.md)

## Deployment 

### Local Deployment

Start up and test a local Kora Server in minutes: [Quick Start Guide](../getting-started/QUICK_START.md)

### Docker

Use the sample Dockerfile to deploy on any container platform:

**[→ Sample Dockerfile](./deploy/sample/Dockerfile)**

### Platform-Specific Guides

- **[Railway Deployment](deploy/RAILWAY.md)** 

*More integration guides coming soon*


## Need Help?

- **[Solana Stack Exchange](https://solana.stackexchange.com/)** - Ask questions/share learnings (make sure to use the `kora` tag)
- **[GitHub Issues](https://github.com/solana-foundation/kora/issues)** - Report bugs or get help





