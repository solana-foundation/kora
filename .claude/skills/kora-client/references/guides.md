# Kora Client Guides

## Table of Contents

- [Full Gasless Transaction Flow](#full-gasless-transaction-flow)
- [Jito Bundle Integration](#jito-bundle-integration)
- [x402 Payment Protocol](#x402-payment-protocol)

---

## Full Gasless Transaction Flow

Complete working example: `examples/getting-started/demo/client/src/full-demo.ts`

### Required Imports

```ts
import { KoraClient } from "@solana/kora";
import {
  createKeyPairSignerFromBytes,
  getBase58Encoder,
  createNoopSigner,
  address,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  partiallySignTransaction,
  Blockhash,
  Base64EncodedWireTransaction,
  Instruction,
  KeyPairSigner,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  MicroLamports,
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
} from "@solana/kit";
import { getAddMemoInstruction } from "@solana-program/memo";
import { createRecentSignatureConfirmationPromiseFactory } from "@solana/transaction-confirmation";
import { updateOrAppendSetComputeUnitLimitInstruction, updateOrAppendSetComputeUnitPriceInstruction } from "@solana-program/compute-budget";
```

### Step-by-Step Implementation

#### 1. Initialize Clients

```ts
const client = new KoraClient({ rpcUrl: 'http://localhost:8080/' });
const rpc = createSolanaRpc('http://127.0.0.1:8899');
const rpcSubscriptions = createSolanaRpcSubscriptions('ws://127.0.0.1:8900');
const confirmTransaction = createRecentSignatureConfirmationPromiseFactory({ rpc, rpcSubscriptions });
```

#### 2. Setup Keys

```ts
const senderKeypair = await createKeyPairSignerFromBytes(/* bytes */);
const { signer_address } = await client.getPayerSigner();
```

#### 3. Create Instructions

```ts
// Token transfer via Kora helper
const transferTokens = await client.transferTransaction({
  amount: 10_000_000, // 10 USDC
  token: paymentToken,
  source: sender.address,
  destination: recipient.address,
});

// SOL transfer
const transferSol = await client.transferTransaction({
  amount: 10_000_000, // 0.01 SOL
  token: '11111111111111111111111111111111',
  source: sender.address,
  destination: recipient.address,
});

// Memo (from @solana-program/memo)
const memoIx = getAddMemoInstruction({ memo: 'Hello, Kora!' });

const instructions = [...transferTokens.instructions, ...transferSol.instructions, memoIx];
```

#### 4. Get Payment Instruction

```ts
const noopSigner = createNoopSigner(address(signer_address));
const { blockhash } = await client.getBlockhash();

// Build estimate transaction
const estimateTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({
    blockhash: blockhash as Blockhash, lastValidBlockHeight: 0n,
  }, tx),
  tx => appendTransactionMessageInstructions(instructions, tx),
  tx => updateOrAppendSetComputeUnitPriceInstruction(1_000_000n as MicroLamports, tx),
  tx => updateOrAppendSetComputeUnitLimitInstruction(200_000, tx),
);

const signedEstimate = await partiallySignTransactionMessageWithSigners(estimateTx);
const base64Estimate = getBase64EncodedWireTransaction(signedEstimate);

const { payment_instruction } = await client.getPaymentInstruction({
  transaction: base64Estimate,
  fee_token: paymentToken,
  source_wallet: sender.address,
});
```

#### 5. Build and Sign Final Transaction

```ts
const newBlockhash = await client.getBlockhash();
const finalTx = pipe(
  createTransactionMessage({ version: 0 }),
  tx => setTransactionMessageFeePayerSigner(noopSigner, tx),
  tx => setTransactionMessageLifetimeUsingBlockhash({
    blockhash: newBlockhash.blockhash as Blockhash, lastValidBlockHeight: 0n,
  }, tx),
  tx => appendTransactionMessageInstructions([...instructions, payment_instruction], tx),
  tx => updateOrAppendSetComputeUnitPriceInstruction(1_000_000n as MicroLamports, tx),
  tx => updateOrAppendSetComputeUnitLimitInstruction(200_000, tx),
);

const partiallySigned = await partiallySignTransactionMessageWithSigners(finalTx);
const userSigned = await partiallySignTransaction([sender.keyPair], partiallySigned);
const base64Final = getBase64EncodedWireTransaction(userSigned);
```

#### 6. Kora Co-signs and Submit

```ts
// Option A: Kora signs, you send via Solana RPC
const { signed_transaction } = await client.signTransaction({
  transaction: base64Final,
  signer_key: signer_address,
});
const signature = await rpc.sendTransaction(
  signed_transaction as Base64EncodedWireTransaction,
  { encoding: 'base64' }
).send();
await confirmTransaction({ commitment: 'confirmed', signature, abortSignal: new AbortController().signal });

// Option B: Kora signs AND sends
const result = await client.signAndSendTransaction({ transaction: base64Final });
```

### Troubleshooting

- **Transaction validation fails**: Check `allowed_programs` and `allowed_spl_paid_tokens` in operator's kora.toml
- **Payment instruction fails**: Ensure fresh blockhash, verify Kora payment address has initialized ATAs
- **Signature verification fails**: Ensure all required signers included, transaction not modified after signing

---

## Jito Bundle Integration

Requires Kora 2.2+. Enables gasless Jito bundles (up to 5 atomic transactions).

Server requirements:
- `sign_bundle` and `sign_and_send_bundle` enabled
- `allow_transfer = true` in fee payer policy (for Jito tip)
- `bundle.enabled = true` with Jito block engine URL

```ts
// Jito tip: minimum 1,000 lamports to random Jito tip account
// All transactions in bundle must share the same blockhash
// Kora pays the tip and all transaction fees
const result = await client.signAndSendBundle({ transactions: [...] });
// Returns bundle UUID for tracking
```

**Docs**: https://launch.solana.com/docs/kora/guides/jito-demo

---

## x402 Payment Protocol

x402 enables HTTP 402 (Payment Required) flows using Kora as the payment backend.

Architecture: Client -> Protected API -> Facilitator -> Kora -> Solana

Components:
1. **Kora RPC** (port 8080): gasless transaction service
2. **Facilitator** (port 3000): x402-to-Kora adapter (`/verify`, `/settle`, `/supported`)
3. **Protected API** (port 4021): x402-express middleware
4. **Client**: x402 fetch wrapper

Use cases: AI agent marketplaces, pay-per-use APIs, micropayments.

**Docs**: https://launch.solana.com/docs/kora/guides/x402
