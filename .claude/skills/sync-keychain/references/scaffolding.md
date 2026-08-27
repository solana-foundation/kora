# Scaffolding a signer adapter

Read the upstream `<Name>SignerConfig` struct and the `from_<name>` signature before writing
anything. Kora's config struct mirrors the upstream one, with every secret-bearing field replaced
by a `*_env` field holding the name of the env var to read.

These are templates, not a spec. Prefer the shape of the nearest existing analogue in
`crates/lib/src/signer/config.rs`.

## 1. Config struct

Alongside the other `*SignerConfig` structs:

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

## 2. Enum variant

In `SignerTypeConfig`:

```rust
/// <Name> signer configuration
<Name> {
    #[serde(flatten)]
    config: <Name>SignerConfig,
},
```

## 3. Build function

`async` if and only if `from_<name>` is async upstream:

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

`sanitize_error!` is mandatory here — remote signer SDKs routinely echo the credential back inside
error strings.

## 4. Validation function

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

Both checks earn their place: the empty check catches a missing TOML field, `get_env_var_for_signer`
catches a field that names an env var which is unset at runtime.

## 5. Match arms

`build_signer_from_config` and `validate_individual_signer_config`, both in
`crates/lib/src/signer/config.rs`.

## 6. HTTP config arm

`crates/lib/src/validator/signer_validator.rs` — pass `&config.http_config` if the signer has one,
`&None` otherwise.
