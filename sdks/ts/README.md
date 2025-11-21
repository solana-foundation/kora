# Kora TypeScript SDK

A TypeScript SDK for interacting with the Kora RPC server. This SDK provides a type-safe interface to all Kora RPC methods.

## Development

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


## Quick Start

```typescript
import { KoraClient } from '@solana/kora';

// Initialize the client with your RPC endpoint
const client = new KoraClient({ rpcUrl: 'http://localhost:8080' });

// Example: Transfer tokens
const result = await client.transferTransaction({
  amount: 1000000, // 1 USDC (6 decimals)
  token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC mint
  source: "sourceAddress",
  destination: "destinationAddress"
});

// Access the base64 encoded transaction, base64 encoded message, and parsed instructions directly
console.log('Transaction:', result.transaction);
console.log('Message:', result.message);
console.log('Instructions:', result.instructions);
```

**[→ API Reference](./docs/README.md)**
**[→ Quick Start](/docs/getting-started/QUICK_START.md)**