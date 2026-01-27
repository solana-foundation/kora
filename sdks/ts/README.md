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

Tests rely on [Solana CLI's](https://solana.com/docs/intro/installation) local test validator. 

Run:

```bash
pnpm test:ci:integration
```

This will start a local test validator and run all tests.

