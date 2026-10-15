# @noirwire/relayer

TypeScript client for the NoirWire relayer. Your users transact without holding SOL: the relayer
pays the network fee and the user pays it back in USDC, plus the markup you set for your app in
the console. On devnet nothing is charged.

Not published yet. `@solana/web3.js` 1.x is a peer dependency.

## Use

```ts
import { createRelayer } from "@noirwire/relayer";

const relayer = createRelayer({ app: "app_xxx", network: "devnet" }); // or { app, url }

const feePayer = await relayer.feePayer(); // set it as the transaction's fee payer
transaction.feePayer = feePayer;

const result = await relayer.signAndSend(transaction, wallet); // wallet: { publicKey, signTransaction }
if (result.outcome === "sent") console.log(result.signature);
else console.log("unknown", result.messageHash, result.signature); // see below
```

Build the transaction as you normally would, with its recent blockhash, but with the relayer's fee
payer. `Transaction` and `VersionedTransaction` both work. The relayer checks it, prices it and adds
the USDC payment; the user's wallet signs that exact transaction.

Lower level:

```ts
const { transaction: prepared, quote, expiresAt } = await relayer.prepare(transaction, wallet.publicKey);
// show quote.totalMicroUsdc (6 decimals) to the user, have them sign `prepared`
const result = await relayer.send(signedPrepared);
```

Other calls: `relayer.config()` returns the deployment's network, fee payer, USDC mint, whether it
charges, and the markup and split limits.

## Results and errors

- `signAndSend` and `send` resolve to `{ outcome: "sent", signature, status }` when the relayer's
  node accepted the transaction. That is not confirmation: watch the signature as you would any other.
- They resolve to `{ outcome: "unknown", messageHash, signature? }` when the relayer could not say
  whether it signed or sent the transaction. **Nothing is retried and you must not retry it
  yourself with another fee payer**: it may still land. Look the signature up when there is one.
  Sending the same signed transaction again returns the recorded outcome.
- Once a send has been dispatched, anything that is not a clear answer throws
  `SendOutcomeUnknownError`, which means the same thing as `outcome: "unknown"`: a dropped
  connection, a timeout, a 5xx (a proxy's 502 after the relayer broadcast, say), a body that is not
  JSON, or any reply the relayer does not document. Never retry it with another fee payer.
- A refusal the relayer gives before it signs throws `RelayerError` with a stable `code` (for
  example `app_paused`, `app_settings_changed`, `insufficient_usdc`, `no_quote`) and the HTTP
  `status`: a 4xx with a code, or a 503 with `loss_ceiling`, `reconciliation_unhealthy` or
  `overloaded`. Nothing was signed or sent. For `prepare` every error is a `RelayerError`, and a
  refused `prepare` is safe to try again once the cause is fixed. `app_settings_changed` means the
  app's payout wallet, markup or program list changed since `prepare`; call `prepare` again.

The relayer allows requests from any origin, so it can be called from a browser. The origin list in
the console is hygiene, not protection: anyone can send any `Origin` header.

## Develop

```sh
npm ci
npm run lint    # tsc --noEmit
npm test        # builds to dist, then node --test
```
