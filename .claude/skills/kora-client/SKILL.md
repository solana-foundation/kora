---
name: kora-client
description: "Kora TypeScript SDK and JSON-RPC API integration for Solana gasless transactions, fee abstraction, and Jito bundles. Use when the user asks about: (1) @solana/kora SDK - KoraClient, koraPlugin, gasless transactions, fee estimation, payment instructions, Jito bundles, (2) Kora RPC methods - estimateTransactionFee, estimateBundleFee, signTransaction, signAndSendTransaction, signBundle, signAndSendBundle, getPaymentInstruction, getConfig, getBlockhash, getSupportedTokens, getPayerSigner, getVersion, (3) integrating with a Kora paymaster node from a client application, (4) building gasless transaction flows on Solana, (5) paying Solana fees in SPL tokens like USDC, (6) reCAPTCHA bot protection for Kora. Do NOT use for running/configuring a Kora node (use kora-operator instead)."
---

# Kora Client Integration

Kora is a Solana paymaster: users pay fees in SPL tokens (e.g. USDC) instead of SOL.

**Docs**: https://launch.solana.com/docs/kora/ · **SDK**: `@solana/kora` (npm)
**Peer deps**: `@solana/kit` v6+, `@solana-program/token` v0.10+

## Where things are

| Topic | Reference |
|---|---|
| Per-method params, responses, TS types, error format | [references/rpc-api.md](references/rpc-api.md) |
| Complete worked gasless flow, Jito bundles, x402, troubleshooting | [references/guides.md](references/guides.md) |

## Two client shapes

`KoraClient` is standalone; `koraPlugin` composes into a Kit client and returns Kit types
(`Address`, `Blockhash`) rather than raw strings.

```ts
import { KoraClient } from '@solana/kora';
const client = new KoraClient({ rpcUrl, apiKey, hmacSecret, getRecaptchaToken });

// or
import { createEmptyClient } from '@solana/kit';
import { koraPlugin } from '@solana/kora';
const client = createEmptyClient().use(koraPlugin({ endpoint, apiKey, getRecaptchaToken }));
await client.kora.getConfig();
```

## The transaction flow

Build instructions → build an estimate transaction with a **noop signer** as fee payer →
`getPaymentInstruction()` → rebuild the final transaction with a **fresh blockhash** including the
payment instruction → user partially signs → Kora co-signs via `signTransaction` or
`signAndSendTransaction`.

Worked end-to-end code is in [references/guides.md](references/guides.md).

The parts that trip people up:

- **The estimate transaction is thrown away.** It exists only so Kora can price the transaction.
  The final transaction needs a new blockhash, not the estimate's.
- **Noop signer**: `createNoopSigner(address(signerAddress))` reserves the fee payer slot before
  Kora has signed. Get the address from `getPayerSigner()`.
- **Signing order**: the user signs their own instructions *and* the payment transfer. Kora only
  adds the fee payer signature; it will not fix a missing user signature.
- **`signer_key`**: optional, but pass it when the node runs a multi-signer pool so estimate and
  signature come from the same signer.
- **`user_id`**: required on signing methods when the operator runs `free` pricing with usage
  tracking enabled.

## Lighthouse invalidates your signatures

If the operator enables Lighthouse, `signTransaction` and `signBundle` may return a transaction
with an extra balance-assertion instruction. That changes the message, so any signature already
applied is void: re-sign the returned transaction client-side and submit it yourself.

Lighthouse never applies to `signAndSendTransaction` / `signAndSendBundle` — Kora broadcasts those
itself, so there is no opportunity to re-sign and the assertion is skipped.

## Bundles

`signBundle` / `signAndSendBundle` take up to 5 transactions and execute atomically via Jito.
`sign_only_indices` restricts which ones Kora signs. `estimateBundleFee` prices the set.

## Authentication

| Method | Header | Constructor option |
|---|---|---|
| API key | `x-api-key` | `apiKey` |
| HMAC | `x-timestamp` + `x-hmac-signature` | `hmacSecret` |
| reCAPTCHA v3 | `x-recaptcha-token` | `getRecaptchaToken` callback |

All three can be active together, and the SDK builds the HMAC (SHA256 of `timestamp + JSON body`)
for you. reCAPTCHA is checked only on the methods the operator marked protected, and only after
API key / HMAC pass. `/liveness` always bypasses auth.
