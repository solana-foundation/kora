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

Tests rely on [Solana CLI's](https://solana.com/docs/intro/installation) local test validator. 

Start your local Kora RPC Server from the root project directory: 

```bash
make run
```

Make sure the Kora RPC's payer wallet matches `KORA_ADDRESS` (not required if using the default values). 
Make sure the `TEST_USDC_MINT_SECRET` public key is included in the root's `kora.toml` (not required if using the default values)

Run:

```bash
pnpm test:integration
```

This will start a local test validator and run all tests.

Alternatively, you can start your own test validator and run:

```bash
pnpm test
```

Based on your test file, here's the optional environment configuration section:

You may optionally configure a `.env` file for your tests in `sdks/ts`:

```bash
# Solana Configuration
SOLANA_RPC_URL=
SOLANA_WS_URL=
SOLANA_VALIDATOR_STARTUP_TIME=
SOLANA_VALIDATOR_ARGS=
COMMITMENT=

# Kora API Configuration  
KORA_ADDRESS=
KORA_RPC_URL=

# Token Mint Configuration
TOKEN_DECIMALS=
TOKEN_DROP_AMOUNT=
SOL_DROP_AMOUNT=

# Test Configuration
SENDER_SECRET=your_test_private_key
TEST_USDC_MINT_SECRET=your_test_usdc_mint_address
DESTINATION_ADDRESS=your_test_destination_address
```

All environment variables are optional and have sensible defaults.

## Quick Start

```typescript
import { KoraClient } from '@kora/sdk';

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

## API Reference

### Configuration Methods

#### `getConfig()`
Get the current Kora configuration.
```typescript
const config = await client.getConfig();
```

#### `getPayerSigner()`
Get the payer signer and payment destination.
```typescript
const { signer, payment_destination } = await client.getPayerSigner();
```

#### `getSupportedTokens()`
Get list of supported tokens.
```typescript
const { tokens } = await client.getSupportedTokens();
```

### Transaction Methods

#### `transferTransaction(request: TransferTransactionRequest)`
Create a token transfer transaction.
```typescript
const response = await client.transferTransaction({
  amount: 1000000,
  token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  source: "sourceAddress",
  destination: "destinationAddress"
});
```

#### `signTransaction(request: SignTransactionRequest)`
Sign a transaction.
```typescript
const result = await client.signTransaction({
  transaction: "base58EncodedTransaction"
});
```

#### `signAndSendTransaction(request: SignAndSendTransactionRequest)`
Sign and send a transaction.
```typescript
const result = await client.signAndSendTransaction({
  transaction: "base64EncodedTransaction"
});
```

#### `signTransactionIfPaid(request: SignTransactionIfPaidRequest)`
Sign a transaction if it pays the fee payer.
```typescript
const result = await client.signTransactionIfPaid({
  transaction: "base58EncodedTransaction",
  margin: 0.1, // Optional: 10% margin
});
```

### Utility Methods

#### `getBlockhash()`
Get the latest blockhash.
```typescript
const { blockhash } = await client.getBlockhash();
```

#### `estimateTransactionFee(transaction: string, feeToken: string)`
Estimate transaction fee.
```typescript
const { fee_in_lamports } = await client.estimateTransactionFee(
  "base58EncodedTransaction",
  "feeTokenMint"
);
```

## Error Handling

The SDK throws errors with descriptive messages when RPC calls fail:

```typescript
try {
  await client.transferTransaction(request);
} catch (error) {
  console.error('Transfer failed:', error.message);
  // Example error: "RPC Error -32602: invalid type: map, expected u64"
}
```
