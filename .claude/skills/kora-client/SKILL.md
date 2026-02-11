---
name: kora-client
description: "Kora TypeScript SDK and JSON-RPC API integration for Solana gasless transactions and fee abstraction. Use when the user asks about: (1) @solana/kora SDK - KoraClient, koraPlugin, gasless transactions, fee estimation, payment instructions, (2) Kora RPC methods - estimateTransactionFee, signTransaction, signAndSendTransaction, transferTransaction, getPaymentInstruction, getConfig, getBlockhash, getSupportedTokens, getPayerSigner, (3) integrating with a Kora paymaster node from a client application, (4) building gasless transaction flows on Solana, (5) paying Solana fees in SPL tokens like USDC. Do NOT use for running/configuring a Kora node (use kora-operator instead)."
---

# Kora Client Integration

Kora is a Solana paymaster that enables gasless transactions. Users pay fees in SPL tokens (e.g. USDC) instead of SOL.

**Docs**: https://launch.solana.com/docs/kora/
**SDK**: `@solana/kora` (npm)
**Peer deps**: `@solana/kit` v6+, `@solana-program/token` v0.10+

## Two Client Patterns

### 1. Standalone KoraClient

```ts
import { KoraClient } from '@solana/kora';

const client = new KoraClient({
  rpcUrl: 'https://kora.example.com',
  apiKey: 'optional-api-key',       // x-api-key header
  hmacSecret: 'optional-hmac-secret' // x-timestamp + x-hmac-signature headers
});
```

### 2. Kit Plugin (composable)

```ts
import { createEmptyClient } from '@solana/kit';
import { koraPlugin } from '@solana/kora';

const client = createEmptyClient()
  .use(koraPlugin({ endpoint: 'https://kora.example.com' }));

// Access via client.kora.* - responses use Kit types (Address, Blockhash)
const config = await client.kora.getConfig();
```

## Core Transaction Flow

The gasless transaction pattern has 6 steps:

1. **Build instructions** - Create the user's intended operations
2. **Build estimate tx** - Wrap instructions in a transaction with noop signer as fee payer
3. **Get payment instruction** - Call `getPaymentInstruction()` to get the fee transfer instruction
4. **Build final tx** - Combine user instructions + payment instruction with fresh blockhash
5. **User signs** - User partially signs (authorizes transfers + payment)
6. **Kora co-signs** - Call `signTransaction()` or `signAndSendTransaction()` for Kora's fee payer signature

### Key Concepts

- **Noop signer**: Placeholder for Kora's fee payer when building transactions before Kora signs
  ```ts
  const noopSigner = createNoopSigner(address(signerAddress));
  ```
- **Partial signing**: User signs their parts, Kora adds fee payer signature
- **Fresh blockhash**: Always get a new blockhash for the final transaction
- **`signer_key` param**: Optional on all methods - use when working with multi-signer pools for consistency

## Quick Examples

### Gasless Transfer (Kora pays everything)

```ts
// Create transfer
const { transaction } = await client.transferTransaction({
  amount: 1_000_000,  // 1 USDC (6 decimals)
  token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  source: userWallet.address,
  destination: recipientAddress,
});

// User signs, Kora co-signs and sends
const decoded = transactionFromBase64(transaction);
const signed = await signTransaction([userWallet.keyPair], decoded);
const result = await client.signAndSendTransaction({
  transaction: getBase64EncodedWireTransaction(signed),
});
```

### User Pays Fees in Token

```ts
// 1. Build instructions
const { instructions } = await client.transferTransaction({ amount, token, source, destination });

// 2. Build estimate transaction with noop signer
const { signer_address } = await client.getPayerSigner();
const noopSigner = createNoopSigner(address(signer_address));
const { blockhash } = await client.getBlockhash();

const estimateTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({ blockhash: blockhash as Blockhash, lastValidBlockHeight: 0n }, tx),
  tx => appendTransactionMessageInstructions(instructions, tx),
);
const signedEstimate = await partiallySignTransactionMessageWithSigners(estimateTx);

// 3. Get payment instruction
const { payment_instruction } = await client.getPaymentInstruction({
  transaction: getBase64EncodedWireTransaction(signedEstimate),
  fee_token: usdcMint,
  source_wallet: userWallet.address,
});

// 4. Build final tx with payment instruction appended
const newBlockhash = await client.getBlockhash();
const finalTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({ blockhash: newBlockhash.blockhash as Blockhash, lastValidBlockHeight: 0n }, tx),
  tx => appendTransactionMessageInstructions([...instructions, payment_instruction], tx),
);

// 5. User signs
const partiallySigned = await partiallySignTransactionMessageWithSigners(finalTx);
const userSigned = await partiallySignTransaction([userWallet.keyPair], partiallySigned);

// 6. Kora co-signs and sends
const { signed_transaction } = await client.signTransaction({
  transaction: getBase64EncodedWireTransaction(userSigned),
  signer_key: signer_address,
});
// Send via Solana RPC
await rpc.sendTransaction(signed_transaction as Base64EncodedWireTransaction, { encoding: 'base64' }).send();
```

### Fee Estimation

```ts
const fees = await client.estimateTransactionFee({
  transaction: base64Tx,
  fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
});
// fees.fee_in_lamports, fees.fee_in_token, fees.signer_pubkey, fees.payment_address
```

### SOL Transfer via Kora

Use the native SOL mint address `11111111111111111111111111111111`:

```ts
const { transaction } = await client.transferTransaction({
  amount: 10_000_000,  // 0.01 SOL
  token: '11111111111111111111111111111111',
  source: sender.address,
  destination: recipient.address,
});
```

## RPC Methods Reference

For detailed method specs (params, responses, types), see [references/rpc-api.md](references/rpc-api.md).

## Full Transaction Flow Guide

For a complete step-by-step gasless transaction tutorial, see [references/guides.md](references/guides.md).

## Authentication

Two optional methods (both can be active simultaneously):

| Method | Headers | Config |
|--------|---------|--------|
| API Key | `x-api-key` | `apiKey` in constructor |
| HMAC | `x-timestamp` + `x-hmac-signature` | `hmacSecret` in constructor |

HMAC: SHA256 of `timestamp + JSON body`. SDK handles this automatically.

The `/liveness` endpoint always bypasses authentication.
