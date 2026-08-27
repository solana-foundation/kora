---
name: kora-operator
description: "Kora paymaster node operator guide. Use when the user asks about: configuring kora.toml (rate limits, validation, allowed programs/tokens, fee payer policy, pricing, auth, caching, bundles, lighthouse), setting up signers.toml (memory/Turnkey/Privy/Vault, pool strategies), running Kora (kora rpc start, config validate, CLI, justfile), deploying to Docker/Railway, fee calculation (margin/fixed/free pricing), Jito bundle config, Lighthouse fee payer protection, reCAPTCHA bot protection, Prometheus monitoring, usage limits with rules, or API key/HMAC authentication setup. Not for client SDK integration (use kora-client)."
---

# Kora Node Operator Guide

Run Kora nodes to validate, sign, and sponsor Solana transaction fees for your users.

**Docs**: https://launch.solana.com/docs/kora/operators · **Install**: `cargo install kora-cli`
**Docker**: `ghcr.io/solana-foundation/kora:latest` · **Source**: https://github.com/solana-foundation/kora

## Where things are

| Topic | Reference |
|---|---|
| Every `kora.toml` section and field, production example | [references/configuration.md](references/configuration.md) |
| `signers.toml`: memory, Turnkey, Privy, Vault, pools, multi-signer | [references/signers.md](references/signers.md) |
| Fee components, pricing models, drain vectors | [references/fees.md](references/fees.md) |
| Docker and Railway deployment | [references/deployment.md](references/deployment.md) |

`crates/lib/src/config.rs` is the authority on config shape, and `kora config validate` is the
fastest way to check a real file. Prefer both over any enumeration written down in prose.

## Getting running

Two files are required: `kora.toml` (validation, auth, pricing, caching, methods, bundles,
lighthouse, metrics) and `signers.toml` (keys, types, selection strategy).

```bash
kora --config kora.toml config validate
kora --config kora.toml rpc start --signers-config signers.toml
kora --config kora.toml rpc initialize-atas --signers-config signers.toml
```

`config validate-with-rpc` additionally checks the config against live chain state (that allowed
mints exist, that the fee payer is funded). Worth running before a first deploy.

`initialize-atas` is required before the node can receive token payments — without it the payment
destination has no associated token accounts and transactions fail at execution, not validation.

`--rpc-url` (env `RPC_URL`) overrides the config. `kora rpc start --help` covers port, logging
format, `--no-load-signer`, and the ATA batching flags.

## Decisions that matter

### Pricing, and the drain risk it carries

| Model | Use case | Risk |
|---|---|---|
| `margin` (default) | cost + markup | Safest — prices in fee payer outflow |
| `fixed` | flat fee per transaction | Fee payer can be drained |
| `free` | full sponsorship | Fee payer can be drained |

With `fixed` or `free`, the node is not pricing the SOL it spends, so a transaction that moves fee
payer funds is a direct loss. Keep `allow_transfer = false` on the system and token policies.
[references/fees.md](references/fees.md) works through the vectors.

### Fee payer policy

Controls what the fee payer signer may do inside a submitted transaction. Every flag defaults to
`false`, so an unlisted action is denied — enable only what your flows need, and treat each `true`
as a deliberate acceptance of the corresponding drain vector.

Sections exist for `system` (plus `system.nonce`), `spl_token`, `token_2022`, `alt`,
`bpf_loader_upgradeable`, and `loader_v4`. For the current field list read the policy structs in
`config.rs` rather than trusting a copied table.

### Lighthouse does nothing on broadcast methods

Lighthouse appends a fee-payer balance assertion, which changes the message and requires the client
to re-sign. It is therefore skipped outright when Kora broadcasts: enabling it buys **zero**
protection on `signAndSendTransaction` and `signAndSendBundle`, with no error raised.
`config validate` warns about this; it does not block startup.

If lighthouse is the drain protection, the node must serve `signTransaction` / `signBundle` and the
client must re-sign. Add `L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95` to `allowed_programs`.

### Authentication

API key (`x-api-key`), HMAC (`x-timestamp` + `x-hmac-signature`), and reCAPTCHA v3 can all be
active at once. Each reads from `[kora.auth]` or an env var (`KORA_API_KEY`, `KORA_HMAC_SECRET`,
`KORA_RECAPTCHA_SECRET`). reCAPTCHA runs only after API key and HMAC have passed, and only on
`protected_methods`. `/liveness` bypasses all of it.

### Jito bundles

Atomic multi-transaction execution via `[kora.bundle]` + `[kora.bundle.jito]`. If Kora pays the
Jito tip, that tip is a system transfer from the fee payer, so it needs
`allow_transfer = true` in `[validation.fee_payer_policy.system]` — which reopens the drain vector
above. Pair it with `margin` pricing.

### Usage limits, caching, metrics

Usage limits are rule-based per user with a Redis backend, and require a `user_id` on signing
requests once enabled. `fallback_if_unavailable` decides whether a Redis outage fails open or
closed — the default (`false`) fails closed. Redis account caching and Prometheus metrics at
`/metrics` are both independent opt-ins. Config shapes in
[references/configuration.md](references/configuration.md).
