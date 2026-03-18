---
description: Sync solana-keychain version and generate new signer adapters
allowed-tools: WebFetch, Read, Edit, Grep, Glob, Bash(cargo:*), mcp__repomix__pack_remote_repository, mcp__repomix__grep_repomix_output
---

# Sync solana-keychain Dependency

Execute this workflow to sync solana-keychain and detect new signers:

## Step 1: Fetch Latest Version

Use WebFetch on `https://crates.io/api/v1/crates/solana-keychain`
- Extract `crate.max_version` from JSON response

## Step 2: Check Current Version

Read `crates/lib/Cargo.toml` and find current `solana-keychain` version.

If different: update the version line using Edit tool.

## Step 3: Scan solana-keychain for Signer Types

Use `pack_remote_repository`:
- Remote: `solana-foundation/solana-keychain`
- Include pattern: `**/*.rs`

Then use `grep_repomix_output` to find all signer constructors:
- Pattern: `pub (async )?fn from_\w+`

These `from_*` methods are the available signer backends.

## Step 4: Compare Against Existing Adapters

Read `crates/lib/src/signer/config.rs` and identify existing `SignerTypeConfig` enum variants:
- Memory
- Turnkey
- Privy
- Vault

Identify any NEW signers from Step 3 not in this list.

## Step 5: Generate Scaffolding for New Signers

For each NEW signer:

1. **Analyze the `from_*` method signature** to understand required parameters

2. **Generate config struct** (after line ~91):
```rust
/// [SignerName] signer configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct [SignerName]SignerConfig {
    pub param1_env: String,
    pub param2_env: String,
}
```

3. **Add enum variant** to `SignerTypeConfig`:
```rust
/// [SignerName] signer configuration
[SignerName] {
    #[serde(flatten)]
    config: [SignerName]SignerConfig,
},
```

4. **Add build function** (check if async based on `from_*` signature):
```rust
fn build_[signer_name]_signer(
    config: &[SignerName]SignerConfig,
    signer_name: &str,
) -> Result<Signer, KoraError> {
    let param1 = get_env_var_for_signer(&config.param1_env, signer_name)?;
    Signer::from_[signer_name](param1).map_err(|e| {
        KoraError::SigningError(format!(
            "Failed to create [SignerName] signer '{signer_name}': {}",
            sanitize_error!(e)
        ))
    })
}
```

5. **Add validation function**:
```rust
fn validate_[signer_name]_config(
    config: &[SignerName]SignerConfig,
    signer_name: &str,
) -> Result<(), KoraError> {
    let env_vars = [
        ("param1_env", &config.param1_env),
    ];
    for (field_name, env_var) in env_vars {
        if env_var.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "[SignerName] signer '{signer_name}' must specify non-empty {field_name}"
            )));
        }
    }
    Ok(())
}
```

6. **Update match arms** in `build_signer_from_config` and `validate_individual_signer_config`

## Step 6: Apply Edits

Use Edit tool to insert code into `crates/lib/src/signer/config.rs`:
- Config structs after existing ones
- Enum variants in SignerTypeConfig
- Build/validate functions in SignerConfig impl
- Update match arms

## Step 7: Verify

Run `cargo check -p kora-lib` to verify compilation.

Report summary:
- Version: [old] -> [new] (or "already latest")
- New signers: [list or "none"]
- Files modified: [list]
