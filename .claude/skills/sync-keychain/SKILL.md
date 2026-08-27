---
name: sync-keychain
description: "Sync the solana-keychain dependency and scaffold adapters for signer backends Kora does not support yet. Use when the user says 'sync keychain', 'bump solana-keychain', 'check for new signers', or 'add support for the <X> signer'."
---

# Sync solana-keychain

Bump the `solana-keychain` pin, then reconcile Kora's `SignerTypeConfig` against the backends
upstream actually ships.

## Bump the pin

Compare the crates.io `max_stable_version` against the pin in `crates/lib/Cargo.toml`, edit if they
differ, then `cargo update -p solana-keychain`.

Keep `default-features = false` and the `features = ["all", "sdk-v4"]` list intact. `sdk-v4`
exact-pins `solana-sdk` for the entire workspace, so a major keychain bump cascades into every
crate — expect that to be the bulk of the work on a major.

## Find the gap

Upstream backends, from a shallow clone of `solana-foundation/solana-keychain`:

```bash
grep -rhoE "pub struct [A-Za-z]+SignerConfig" --include="*.rs" "$tmp/kc" \
  | sed 's/pub struct //;s/SignerConfig//' | sort -u
```

Enumerate by `*SignerConfig` struct, **not** by `from_*` method. `from_*` also matches key-format
helpers (`from_bytes`, `from_pem`, `from_u8_array_string`, …) which are not backends and would be
scaffolded as phantom signers.

Kora's side — derive it, never hardcode:

```bash
awk '/pub enum SignerTypeConfig/,/^}/' crates/lib/src/signer/config.rs \
  | grep -E "^    [A-Z][A-Za-z]* \{" | tr -d ' {'
```

Variant names are the upstream struct prefix: `AwsKmsSignerConfig` → `AwsKms` → `from_aws_kms`.
Also record whether each `from_<name>` is `async`; the build function must match.

The difference between the two lists is the work. If it's empty, report and stop.

## Scaffold what's missing

Six edits per signer, five of them in `crates/lib/src/signer/config.rs`: config struct, enum
variant, build function, validation function, two match arms — plus an arm in
`crates/lib/src/validator/signer_validator.rs`. That last match is exhaustive, so omitting it is a
compile error rather than a silent gap.

Copy the shape of the nearest existing analogue (`Fireblocks` for an async HTTP backend, `AwsKms`
for a cloud-KMS one). [references/scaffolding.md](references/scaffolding.md) has the templates and
the reasoning behind each piece.

Two rules that are not obvious from the surrounding code:

- Secrets are referenced by env var *name* in `signers.toml`, never inlined. Any new config field
  holding a credential must be named `*_env`.
- Never interpolate an upstream error directly. `sanitize_error!` strips secrets out of error
  strings from remote signer SDKs.

Verify with `cargo check -p kora-lib`, `just fmt`, `cargo test -p kora-lib --lib signer`.

## Report

Version before/after, upstream backend count, signers scaffolded, files touched. Say plainly that
scaffolding is a starting point: a new signer lands with no integration test and no `signers.toml`
documentation entry.
