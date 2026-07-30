# kora-fuzz

Coverage-guided fuzzing for Kora's untrusted-input paths, using [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer). This is a standalone workspace so the sanitizer build flags don't leak into the main workspace.

Kora runs off-chain (native Rust), so the fuzzable surface is the code that turns bytes from a JSON-RPC client into typed instructions — not on-chain sBPF, which is why an SVM fuzzer like Crucible does not apply here.

## Setup

```bash
cargo install cargo-fuzz   # nightly toolchain required (already pinned in rust-toolchain.toml)
```

## Targets

- `parse_transaction` — raw bytes → `bincode` `VersionedTransaction` → `from_kora_built_transaction` → every `get_or_parse_*` instruction parser. Finds panics in instruction decoding (out-of-bounds indexing, bad discriminators).
- `decode_b64_transaction` — arbitrary strings → `TransactionUtil::decode_b64_transaction`. Exercises the base64 + `bincode` decode entry point used by the RPC layer.

## Running

```bash
just fuzz parse_transaction              # or: cd fuzz && cargo fuzz run parse_transaction
just fuzz parse_transaction -max_total_time=60
just fuzz-list
```

A crash writes a reproducer to `fuzz/artifacts/<target>/`; re-run it with `cargo fuzz run <target> fuzz/artifacts/<target>/<crash-file>`.

## CI

`.github/workflows/fuzz.yml` runs two jobs:

- **smoke** (every PR/push touching `crates/**` or `fuzz/**`): `cargo fuzz build` + a 60s run per target. Fails on a build break or a crash; uploads any reproducer as an artifact.
- **deep** (nightly cron + manual `workflow_dispatch`): 10 min per target, uploads the corpus and any crashes as artifacts.

## Property tests

Structural invariants (e.g. fee-payer drain safety across the policy matrix) live as `proptest` cases in the `kora-lib` unit tests, not here — see `crates/lib/src/validator/transaction_validator.rs` (`mod fee_payer_policy_props`). Run with `cargo test -p kora-lib --lib fee_payer_policy_props`.
