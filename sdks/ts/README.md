# Kora TypeScript SDK

A TypeScript SDK for interacting with the Kora RPC server. This SDK provides a type-safe interface to all Kora RPC methods (requires a Kora RPC server to be running).


## Installation

```bash
pnpm install @solana/kora
```

## Quick Start

```typescript
import { KoraClient } from '@solana/kora';

// Initialize the client with your RPC endpoint
const client = new KoraClient({ rpcUrl: 'http://localhost:8080' });

// Example: Get Kora to sign a transaction
const result = await client.signTransaction({
  transaction: 'myBase64EncodedTransaction'
});

// Access the signed transaction (base64 encoded)
console.log('Signed transaction:', result.signed_transaction);
```

**[→ API Reference](https://launch.solana.com/docs/kora/json-rpc-api)**
**[→ Quick Start](https://launch.solana.com/docs/kora/getting-started/quick-start)**

## Local Development

### Building from Source

```bash
# Install dependencies
pnpm install

# Build the SDK
pnpm run build
```

### Running Tests


Start your local Kora RPC Server from the root project directory: 

```bash
kora --config tests/src/common/fixtures/kora-test.toml rpc start --signers-config tests/src/common/fixtures/signers.toml
```

Most of the suite runs its chain in-process, but a few tests reach a Solana node
at `SOLANA_RPC_URL` (default `http://127.0.0.1:8899`), so start one alongside Kora.

Run:

```bash
pnpm test:integration
```

This suite is not run in CI: it has no automated way to boot a node yet, and is
waiting on [surfpool#804](https://github.com/solana-foundation/surfpool/pull/804)
so it can embed a surfnet the way the Rust integration tests do.

