# Kora TypeScript SDK

A TypeScript SDK for interacting with the Kora RPC server. This SDK provides a type-safe interface to all Kora RPC methods.

## Installation

```bash
pnpm install @kora/sdk
```

## Quick Start

```typescript
import { KoraClient } from '@kora/sdk';

// Initialize the client with your RPC endpoint
const client = new KoraClient('http://localhost:8080');

// Example: Transfer tokens
const signature = await client.transferTransaction({
  amount: 1000000, // 1 USDC (6 decimals)
  token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC mint
  source: "sourceAddress",
  destination: "destinationAddress"
});
```

## API Reference

### Configuration Methods

#### `getConfig()`
Get the current Kora configuration.
```typescript
const config = await client.getConfig();
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

## Development

### Building from Source

```bash
# Install dependencies
pnpm install

# Build the SDK
pnpm run build

# Run tests
pnpm test
```

### Running Tests

Tests require a valid devnet wallet address. Update the `/ts-sdk/test/setup.ts` file with the correct env variables:

```env
KORA_RPC_URL=http://localhost:8080
TEST_WALLET_PUBKEY=your_devnet_wallet_address
TEST_WALLET_PRIVATE_KEY=your_local_wallet_private_key_for_tests
USDC_MINT=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
```

Then run:
```bash
pnpm test
```
