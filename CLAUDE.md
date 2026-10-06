# CLAUDE.md

Kora is a Solana paymaster node: clients pay transaction fees in SPL tokens instead of SOL.

Crates: `lib` (core plus the RPC server, which has no crate of its own — it lives at
`crates/lib/src/rpc_server/`), `cli`, and `kora-deploy` (versioned independently of the other two).
Plus `tests/` and `sdks/ts/`.

Commands are in the `justfile`. Config schema is `crates/lib/src/config.rs`. Read those directly.

## Layout

`validator/transaction_validator.rs` is ~6500 lines, and its logic sits above a very large inline
test module starting at ~500. Read the logic range, not the whole file. The fee-payer policy checks
it dispatches to live beside it in `validator/transaction_validator/`, one file per program
(`system.rs`, `spl_token.rs`, `token_2022.rs`, `alt.rs`, `loader_v4.rs`,
`bpf_loader_upgradeable.rs`), each a list of `deny_fee_payer!` calls (`validator/macros.rs`).

Instruction parsing lives in `transaction/instruction_util/`, one file per program (`system.rs`,
`spl_token.rs`, `alt.rs`, `loader_v4.rs`, `bpf_loader_upgradeable.rs`), plus `reconstruct.rs`,
which rebuilds raw instructions from the jsonParsed inner instructions simulation returns. Its
tests are in `tests.rs`.

Fee-payer-policy drain-safety property tests are the exception to that inline pattern: they are
split one file per gated program under
`validator/transaction_validator/fee_payer_policy_props/`. Run them with
`cargo test -p kora-lib --lib fee_payer_policy_props`.

## Gotchas

### Lighthouse silently no-ops on signAndSend

`LighthouseUtil::add_fee_payer_assertion` returns early when `will_send` is true
(`lighthouse/assertion.rs`). Enabling lighthouse therefore buys zero fee-payer drain protection on
`signAndSendTransaction` and `signAndSendBundle` — no error, no assertion instruction. The
assertion mutates the message, so it can only be appended on paths where the client re-signs
afterwards: `signTransaction` and `signBundle`.

`validator/config_validator/transactions.rs` warns when lighthouse is enabled alongside those
methods. It is a warning, not an error; the node still starts unprotected.

### Two drain guards are not flag-gated

Do not go looking for a `fee_payer_policy` flag for these. They are `deny_fee_payer!` calls with no
`unless` clause:

- BPF Loader Upgradeable `Close` (`transaction_validator/bpf_loader_upgradeable.rs`): a fee-payer
  authority paired with a foreign recipient is always rejected.
- Loader v4 `SetProgramLength` (`transaction_validator/loader_v4.rs`): when the fee payer is the
  authority, the recipient must also be the fee payer.

### Fee payer policy must fail closed

Every flag defaults to `false` and the structs carry `#[serde(default)]`. That combination is
deliberate: adding a newly gated instruction must not break an existing operator's config, and it
must land as a denial rather than an accidental allowance. Preserve both when adding a flag.

### V0 transactions need lookup tables resolved before validation

`VersionedTransaction::get_all_account_keys()` returns only the static keys. Validating or pricing a
V0 transaction without resolving first silently skips the lookup-table accounts and under-charges.
Call `VersionedTransactionResolved::resolve_addresses(rpc_client).await` first; it caches. Resolution
costs an RPC call, which is why it is explicit at the call site instead of happening implicitly.

### Private key parsing tries the filesystem first

`KeypairUtil::from_private_key_string` order is: `fs::read_to_string` → `[..]` u8 array → base58
fallback. A key string that happens to match an existing path is read as a file.

### Middleware order is load-bearing

In `rpc_server/server.rs` the reCAPTCHA layer is added last, making it innermost, so it runs only
after API key and HMAC auth have passed. Moving it earlier means unauthenticated traffic burns
reCAPTCHA quota. `/liveness` is proxied ahead of the auth layers and bypasses them.

### Each integration test binary owns one node

`tests/` declares one `[[test]]` target per phase, and each boots its own embedded surfnet plus Kora
node through `harness_context`, seeded from scratch. Kora keeps config in process-global state, so
one config per binary is the constraint: a new phase needs a new target, not a new module in an
existing one. Run a single phase with `cargo test -p tests --test <target>`.

## Conventions

- CLI command output uses `println!`. `log::*` is for the server.
- Errors from external services are wrapped in `sanitize_error!` before they reach a log line or an
  RPC response.
