# Kora

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/solana-foundation/kora)
[![Crates.io](https://img.shields.io/crates/v/kora-rpc.svg)](https://crates.io/crates/kora-rpc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Kora eliminates the SOL requirement for Solana transactions.** Let your users pay fees in any token—USDC, BONK, or your app's native token—while you handle the SOL behind the scenes.

### Why Kora?

- **Better UX**: Users never need SOL  
- **Revenue Control**: Collect fees in USDC, your token, or anything else  
- **Production Ready**: Secure validation, rate limiting, monitoring built-in  
- **Easy Integration**: JSON-RPC API + TypeScript SDK  
- **Flexible Deployment**: Railway, Docker, or any cloud platform

### Architecture

- **Language**: Rust with TypeScript SDK
- **Protocol**: JSON-RPC 2.0  
- **Signers**: Solana Private Key, Turnkey, Privy
- **Authentication**: API Key, HMAC, or none
- **Deployment**: Flexible deployment options (Docker, Railway, etc.) 

### Features

- Configurable validation rules and allowlists
- Rate limiting protection
- Secure key management (Turnkey, Privy, Vault)
- HMAC and API key request authentication
- Transaction spend limits


## Quick Start

Install Kora: 

```bash
cargo install kora-rpc
```

Basic usage:

```bash
kora-rpc -- [OPTIONS] # --help for full list of options
```

**[→ Full Documentation](docs/README.md)** - Learn how Kora works

**[→ Quick Start Guide](docs/getting-started/QUICK_START.md)** - Get Kora running locally minutes

**[→ Node Operator Guide](docs/operators/README.md)** - Run a paymaster


## TypeScript SDK

Kora provides a simple JSON-RPC interface:

```typescript
// Initialize Kora client
import { KoraClient } from '@kora/sdk';
const kora = new KoraClient({ rpcUrl: 'http://localhost:8080' });

// Sign transaction as paymaster
const signed = await kora.signTransaction({ transaction });
```

**[→ API Reference](./sdks/ts/README.md)**

## Local Development

### Prerequisites

- Rust 1.86+ or 
- Solana CLI 2.2+
- Node.js 20+ and pnpm (for SDK)

### Installation

```bash
git clone https://github.com/solana-foundation/kora.git
cd kora
make install
```

### Build

```bash
make build
```

### Running the Server

Basic usage:

```bash
kora-rpc -- [OPTIONS]
```

Or for running with a test configuration, run: 

```bash
make run
```

### Local Testing

To run the tests locally, you need to set up a local validator and test environment.

```bash
solana-test-validator -r
```

And run integration tests:

```bash
make test-integration
```

## Repository Structure

```
kora/
├── crates/                   # Rust workspace
│   ├── kora-lib/             # Core library (signers, validation, transactions)
│   ├── kora-rpc/             # JSON-RPC server implementation
│   ├── kora-cli/             # Command-line interface
│   ├── kora-turnkey/         # Turnkey signer integration
│   └── kora-privy/           # Privy signer integration
├── sdks/                     # Client SDKs
│   ├── ts/                   # TypeScript SDK
├── tests/                    # Integration tests
├── docs/                     # Documentation
│   ├── getting-started/      # Quick start guides
│   └── operators/            # Node operator documentation
├── Makefile                  # Build and development commands
└── kora.toml                 # Example configuration
```

## Community & Support

- **Questions?** Ask on [Solana Stack Exchange](https://solana.stackexchange.com/) (use the `kora` tag)
- **Issues?** Report on [GitHub Issues](https://github.com/solana-foundation/kora/issues)

## Other Resources

- [Kora-RPC Crates.io](https://crates.io/crates/kora-rpc) - Rust crate for running a Kora node
- @kora/sdk NPM Package Coming Soon

---

Built and maintained by the [Solana Foundation](https://solana.org).

Licensed under MIT. See [LICENSE](LICENSE) for details.