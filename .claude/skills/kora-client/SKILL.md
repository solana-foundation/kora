---
name: kora-client
description: "Kora TypeScript SDK and JSON-RPC API integration for Solana gasless transactions, fee abstraction, and Jito bundles. Use when the user asks about: (1) @solana/kora SDK - KoraClient, koraPlugin, gasless transactions, fee estimation, payment instructions, Jito bundles, (2) Kora RPC methods - estimateTransactionFee, estimateBundleFee, signTransaction, signAndSendTransaction, signBundle, signAndSendBundle, getPaymentInstruction, getConfig, getBlockhash, getSupportedTokens, getPayerSigner, getVersion, (3) integrating with a Kora paymaster node from a client application, (4) building gasless transaction flows on Solana, (5) paying Solana fees in SPL tokens like USDC, (6) reCAPTCHA bot protection for Kora. Do NOT use for running/configuring a Kora node (use kora-operator instead)."
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
  apiKey: 'optional-api-key',           // x-api-key header
  hmacSecret: 'optional-hmac-secret',   // x-timestamp + x-hmac-signature headers
  getRecaptchaToken: async () => {      // optional: reCAPTCHA v3 bot protection
    return await grecaptcha.execute('site-key', { action: 'sign' });
  },
});
```

### 2. Kit Plugin (composable)

```ts
import { createEmptyClient } from '@solana/kit';
import { koraPlugin } from '@solana/kora';

const client = createEmptyClient()
  .use(koraPlugin({
    endpoint: 'https://kora.example.com',
    apiKey: 'optional',
    getRecaptchaToken: async () => token,
  }));

// Access via client.kora.* - responses use Kit types (Address, Blockhash)
const config = await client.kora.getConfig();
```

## Core Transaction Flow

The gasless transaction pattern has 6 steps:

1. **Build instructions** - Create the user's intended operations using `@solana-program/*` libraries
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
- **`user_id` param**: Optional on signing methods - required when pricing is `free` and usage tracking is enabled

## Quick Examples

### User Pays Fees in Token

```ts
import { getTransferInstruction, findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';
import { getTransferSolInstruction } from '@solana-program/system';

// 1. Build instructions using @solana-program libraries
const [sourceAta] = await findAssociatedTokenPda({ owner: sender.address, mint: address(usdcMint), tokenProgram: TOKEN_PROGRAM_ADDRESS });
const [destAta] = await findAssociatedTokenPda({ owner: recipient.address, mint: address(usdcMint), tokenProgram: TOKEN_PROGRAM_ADDRESS });
const tokenTransferIx = getTransferInstruction({ source: sourceAta, destination: destAta, authority: sender, amount: 10_000_000n });

// 2. Build estimate transaction with noop signer
const { signer_address } = await client.getPayerSigner();
const noopSigner = createNoopSigner(address(signer_address));
const { blockhash } = await client.getBlockhash();

const estimateTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({ blockhash: blockhash as Blockhash, lastValidBlockHeight: 0n }, tx),
  tx => appendTransactionMessageInstructions([tokenTransferIx], tx),
);
const signedEstimate = await partiallySignTransactionMessageWithSigners(estimateTx);

// 3. Get payment instruction
const { payment_instruction } = await client.getPaymentInstruction({
  transaction: getBase64EncodedWireTransaction(signedEstimate),
  fee_token: usdcMint,
  source_wallet: sender.address,
});

// 4. Build final tx with payment instruction appended
const newBlockhash = await client.getBlockhash();
const finalTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({ blockhash: newBlockhash.blockhash as Blockhash, lastValidBlockHeight: 0n }, tx),
  tx => appendTransactionMessageInstructions([tokenTransferIx, payment_instruction], tx),
);

// 5. User signs
const partiallySigned = await partiallySignTransactionMessageWithSigners(finalTx);
const userSigned = await partiallySignTransaction([sender.keyPair], partiallySigned);

// 6. Kora co-signs and sends
const result = await client.signAndSendTransaction({
  transaction: getBase64EncodedWireTransaction(userSigned),
  signer_key: signer_address,
});
```

### Fee Estimation

```ts
const fees = await client.estimateTransactionFee({
  transaction: base64Tx,
  fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
});
// fees.fee_in_lamports, fees.fee_in_token, fees.signer_pubkey, fees.payment_address
```

### Jito Bundle

```ts
// Sign and submit bundle atomically via Jito (max 5 transactions)
const { bundle_uuid, signed_transactions } = await client.signAndSendBundle({
  transactions: [base64Tx1, base64Tx2, base64Tx3],
  signer_key: signerAddress,
});

// Or sign without submitting (for client-side Jito submission)
const { signed_transactions } = await client.signBundle({
  transactions: [base64Tx1, base64Tx2],
  sign_only_indices: [0],  // only sign first transaction
});

// Estimate bundle fees
const estimate = await client.estimateBundleFee({
  transactions: [base64Tx1, base64Tx2],
  fee_token: usdcMint,
});
```

### Lighthouse Re-sign Flow

When the operator has Lighthouse fee payer protection enabled, `signTransaction`/`signBundle` may modify the transaction message (adding a balance assertion). Client signatures become invalid and must be re-applied:

```
signTransaction → client receives modified tx → client re-signs → client sends to network
```

Note: Lighthouse does NOT apply to `signAndSendTransaction`/`signAndSendBundle` (broadcast methods).

## RPC Methods Reference

For detailed method specs (params, responses, types), see [references/rpc-api.md](references/rpc-api.md).

## Full Transaction Flow Guide

For a complete step-by-step gasless transaction tutorial, see [references/guides.md](references/guides.md).

## Authentication

Three optional methods (all can be active simultaneously):

| Method | Headers | Config |
|--------|---------|--------|
| API Key | `x-api-key` | `apiKey` in constructor |
| HMAC | `x-timestamp` + `x-hmac-signature` | `hmacSecret` in constructor |
| reCAPTCHA v3 | `x-recaptcha-token` | `getRecaptchaToken` callback in constructor |

HMAC: SHA256 of `timestamp + JSON body`. SDK handles this automatically.
reCAPTCHA: Token auto-attached via callback. Only verified on server-configured protected methods (signing methods by default).

The `/liveness` endpoint always bypasses authentication.
