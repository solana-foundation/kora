# CLAUDE.md

Kora is a Solana paymaster node: clients pay transaction fees in SPL tokens instead of SOL.

Crates: `lib` (core plus the RPC server, which has no crate of its own — it lives at
`crates/lib/src/rpc_server/`), `cli`, and `kora-deploy` (versioned independently of the other two).
Plus `tests/` and `sdks/ts/`.

Commands are in the `justfile`. Config schema is `crates/lib/src/config.rs`. Read those directly.

## Layout

Two files are a quarter of the crate, and in both the logic sits above a very large inline test
module. Read the logic range, not the whole file:

| File | Lines | `mod tests` starts |
|------|-------|--------------------|
| `transaction/instruction_util.rs` | ~7300 | ~3870 |
| `validator/transaction_validator.rs` | ~7200 | ~1050 |

Fee-payer-policy drain-safety property tests are the exception to that inline pattern: they are
split one file per gated program under
`validator/transaction_validator/fee_payer_policy_props/`. Token-2022 is the only program still
uncovered. Run them with `cargo test -p kora-lib --lib fee_payer_policy_props`.

## Gotchas

### Lighthouse silently no-ops on signAndSend

`LighthouseUtil::add_fee_payer_assertion` returns early when `will_send` is true
(`lighthouse/assertion.rs`). Enabling lighthouse therefore buys zero fee-payer drain protection on
`signAndSendTransaction` and `signAndSendBundle` — no error, no assertion instruction. The
assertion mutates the message, so it can only be appended on paths where the client re-signs
afterwards: `signTransaction` and `signBundle`.

`config_validator.rs` warns when lighthouse is enabled alongside those methods. It is a warning,
not an error; the node still starts unprotected.

### Two drain guards are not flag-gated

Do not go looking for a `fee_payer_policy` flag for these. They are unconditional in
`validator/transaction_validator.rs`:

- BPF Loader Upgradeable `Close`: a fee-payer authority paired with a foreign recipient is always
  rejected.
- Loader v4 `SetProgramLength`: when the fee payer is the authority, the recipient must also be the
  fee payer.

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

### Integration tests cache their accounts

`just integration-test` reuses previously created test accounts across runs. After changing fixtures
or account setup, pass `--force-refresh` or you will debug a stale account instead of your change.

## Conventions

- CLI command output uses `println!`. `log::*` is for the server.
- Errors from external services are wrapped in `sanitize_error!` before they reach a log line or an
  RPC response.
