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
- `validate_transaction`: runs the full `TransactionValidator::validate_transaction` against a mock RPC client. The input is an `arbitrary`-derived `Scenario` (`src/scenario.rs`): a random `FeePayerPolicy` (every flag drawn from the input), a message version, and up to six structurally valid instructions for System, SPL Token, Token-2022, ATA, ALT, BPF Loader Upgradeable, and Loader v4, built with the upstream interface crates, with the fee payer randomly placed in their account and data slots. Raw instructions for the same programs are mixed in for malformed input. The oracle (`src/oracle.rs`) decides from the generated instruction alone, not Kora's parsers, whether the fee payer holds a gated role whose flag is off (or hits an unconditional drain guard), and the target panics if validation then returns `Ok`. `src/` is a library so later targets can reuse the generator.
- `differential_litesvm`: runs the same `Scenario` through Kora's validator and a [LiteSVM](https://github.com/LiteSVM/litesvm) execution (sigverify and blockhash checks off, bundled SPL Token, Token-2022, ATA and ALT programs). The input also seeds the three non-fee-payer pool keys as wallets, mints, token accounts, or nonce accounts and picks a finite `max_allowed_lamports`. The validator's RPC client is answered from the same LiteSVM state (`getAccountInfo`, rent, and `simulateTransaction` with LiteSVM's inner instructions), so it goes through the production `from_transaction` path. When the validator accepts, the target panics if the lamports held by the fee payer and the accounts it controls (token accounts it owns, nonces it is authority of) dropped by more than the fee plus the cap, or if the fee payer's own account, token accounts, mints, or nonces changed in a way whose policy flag is off (reassigned, closed, delegated, authority changed, drained, inflated).

## Running

```bash
just fuzz parse_transaction              # or: cd fuzz && cargo fuzz run parse_transaction
just fuzz parse_transaction -max_total_time=60
just fuzz-list
```

A crash writes a reproducer to `fuzz/artifacts/<target>/`; re-run it with `cargo fuzz run <target> fuzz/artifacts/<target>/<crash-file>`.

## Seed corpus

`fuzz/corpus/<target>/seed_*` holds committed seeds that `cargo fuzz run` (and CI) loads by default: legacy and V0 transactions covering System (incl. nonce), SPL Token (incl. p-token batch), Token-2022, ALT, BPF Loader Upgradeable, and Loader v4 instructions. `decode_b64_transaction` gets a base64-encoded subset. Entries libFuzzer adds during a run are gitignored; only `seed_*` files are tracked.

`differential_litesvm/seed_freeze_by_fee_payer` is not generated: it is a fuzzer-found input that reaches a successful `FreezeAccount` signed by the fee payer as freeze authority, so CI always exercises the frozen-account check. Its bytes follow the target's `Input` layout; if that layout changes, find a replacement with a temporary target that panics on the same precondition.

Regenerate after changing `examples/gen_seed_corpus.rs` (output is deterministic):

```bash
just fuzz-seeds   # or: cd fuzz && cargo run --example gen_seed_corpus
```

## CI

`.github/workflows/fuzz.yml` runs two jobs:

- **smoke** (every PR/push touching `crates/**` or `fuzz/**`): `cargo fuzz build` + a 60s run per target. Fails on a build break or a crash; uploads any reproducer as an artifact.
- **deep** (nightly cron + manual `workflow_dispatch`): 10 min per target, uploads the corpus and any crashes as artifacts.

## Property tests

Structural invariants (e.g. fee-payer drain safety across the policy matrix) live as `proptest` cases in the `kora-lib` unit tests, not here — see `crates/lib/src/validator/transaction_validator/fee_payer_policy_props/`, one file per gated program type implementing the `DrainRole` trait. System, SPL Token, ALT, BPF Loader Upgradeable, and Loader v4 are covered; Token-2022 is open work. Run with `cargo test -p kora-lib --lib fee_payer_policy_props`.
