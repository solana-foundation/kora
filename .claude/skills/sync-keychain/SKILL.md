---
name: sync-keychain
description: "Sync the solana-keychain dependency and scaffold adapters for signer backends Kora does not support yet. Use when the user says 'sync keychain', 'bump solana-keychain', 'check for new signers', or 'add support for the <X> signer'."
---

# Sync solana-keychain

Bump the `solana-keychain` pin and reconcile Kora's `SignerTypeConfig` against the signer backends upstream actually ships.

---

## Step 1 — Compare versions

```bash
latest=$(curl -sS -H "User-Agent: kora-sync-keychain" \
  https://crates.io/api/v1/crates/solana-keychain | jq -r '.crate.max_stable_version')
current=$(grep -oE 'solana-keychain = \{ version = "[^"]+"' crates/lib/Cargo.toml \
  | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')

echo "current=$current latest=$latest"
```

If they differ, Edit the version string in `crates/lib/Cargo.toml` (the `solana-keychain` entry), then run `cargo update -p solana-keychain`.

Keep `default-features = false` and the `features = ["all", "sdk-v4"]` list intact. `sdk-v4` exact-pins `solana-sdk` for the whole workspace, so a major bump may cascade.

---

## Step 2 — Enumerate upstream backends

```bash
tmp=$(mktemp -d)
git clone --depth 1 -q https://github.com/solana-foundation/solana-keychain "$tmp/kc"
grep -rhoE "pub struct [A-Za-z]+SignerConfig" --include="*.rs" "$tmp/kc" \
  | sed 's/pub struct //;s/SignerConfig//' | sort -u
```

Enumerate by `*SignerConfig` struct, not by `from_*` method. `from_*` also matches key-format helpers (`from_bytes`, `from_pem`, `from_u8_array_string`, `from_private_key_file`, …) that are not backends and would be scaffolded as phantom signers.

For each backend, record whether its constructor is async:

```bash
grep -rhoE "pub (async )?fn from_\w+" --include="*.rs" "$tmp/kc" | sort -u
```

Delete `$tmp` when done.

---

## Step 3 — Enumerate Kora's variants

Derive this list, never hardcode it:

```bash
awk '/pub enum SignerTypeConfig/,/^}/' crates/lib/src/signer/config.rs \
  | grep -E "^    [A-Z][A-Za-z]* \{" | tr -d ' {'
```

Variant names are the upstream struct prefix in PascalCase: `AwsKmsSignerConfig` → `AwsKms` → `from_aws_kms`.

The difference between Step 2 and Step 3 is the work. If it is empty, report and stop.

---

## Step 4 — Scaffold each missing signer

Read the upstream `<Name>SignerConfig` struct and the `from_<name>` signature first. Kora's config struct mirrors the upstream one, with secret-bearing fields replaced by `*_env` names holding the env var to read.

Copy the shape of the nearest existing analogue in `crates/lib/src/signer/config.rs` (`Fireblocks` for an async HTTP backend, `AwsKms` for a cloud-KMS one) rather than the templates below verbatim.

**1. Config struct** — alongside the other `*SignerConfig` structs:

```rust
/// <Name> signer configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct <Name>SignerConfig {
    pub api_key_env: String,
    #[serde(default)]
    pub api_base_url: Option<String>,
    #[serde(default)]
    pub http_config: Option<RemoteSignerHttpConfig>,
}
```

Include `http_config` only if the upstream config has an `http_client_config` field.

**2. Enum variant** in `SignerTypeConfig`:

```rust
/// <Name> signer configuration
<Name> {
    #[serde(flatten)]
    config: <Name>SignerConfig,
},
```

**3. Build function** — `async` iff `from_<name>` is async:

```rust
async fn build_<name>_signer(
    config: &<Name>SignerConfig,
    signer_name: &str,
) -> Result<Signer, KoraError> {
    let api_key = get_env_var_for_signer(&config.api_key_env, signer_name)?;

    let keychain_config = solana_keychain::<Name>SignerConfig {
        api_key,
        api_base_url: config.api_base_url.clone(),
        http_client_config: config
            .http_config
            .as_ref()
            .map(solana_keychain::HttpClientConfig::from),
    };

    Signer::from_<name>(keychain_config).await.map_err(|e| {
        KoraError::SigningError(format!(
            "Failed to create <Name> signer '{signer_name}': {}",
            sanitize_error!(e)
        ))
    })
}
```

Never interpolate `e` directly; `sanitize_error!` strips secrets out of upstream error strings.

**4. Validation function**:

```rust
fn validate_<name>_config(
    config: &<Name>SignerConfig,
    signer_name: &str,
) -> Result<(), KoraError> {
    let env_vars = [("api_key_env", &config.api_key_env)];

    for (field_name, env_var) in env_vars {
        if env_var.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "<Name> signer '{signer_name}' must specify non-empty {field_name}"
            )));
        }
        get_env_var_for_signer(env_var, signer_name)?;
    }
    Ok(())
}
```

Both checks matter: the empty check catches a missing TOML field, `get_env_var_for_signer` catches a named env var that is unset.

**5. Match arms** — `build_signer_from_config` and `validate_individual_signer_config`, both in `crates/lib/src/signer/config.rs`.

**6. HTTP config arm** in `crates/lib/src/validator/signer_validator.rs` — `&config.http_config` if the signer has one, `&None` otherwise. This match is exhaustive, so omitting it is a compile error.

---

## Step 5 — Verify

```bash
cargo check -p kora-lib
just fmt
cargo test -p kora-lib --lib signer
```

---

## Step 6 — Report

- Version: `<old>` → `<new>`, or already latest
- Upstream backends: count
- New signers scaffolded, or none
- Files modified

---

## Notes

- Scaffolding is a starting point, not a finished adapter. Flag to the user that a new signer has no integration test and no `signers.toml` docs entry.
- Secrets are referenced by env var name in `signers.toml`, never inlined. A new config field holding a credential must be named `*_env`.
