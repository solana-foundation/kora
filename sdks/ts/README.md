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

Unit tests need nothing running:

```bash
pnpm test:unit
```

The integration suite talks to a Kora node over `KORA_RPC_URL` and to that
node's Solana RPC over `SOLANA_RPC_URL` (plus `SOLANA_WS_URL` when the
websocket is not the RPC URL with an `http` to `ws` swap). Rather than start
both by hand, run the phase from the workspace root and let the Rust harness
boot a surfnet, seed it, and start Kora:

```bash
cargo test -p tests --test typescript_basic
cargo test -p tests --test typescript_auth
cargo test -p tests --test typescript_free
```

Each flavor is a separate phase because each needs its own Kora config. To run
the suite against a node you started yourself:

```bash
KORA_RPC_URL=http://127.0.0.1:8080 SOLANA_RPC_URL=http://127.0.0.1:8899 pnpm test:integration
```

Setup seeds the mint and wallets itself when they are absent, so this works
against a bare validator as well as against the seeded harness.

