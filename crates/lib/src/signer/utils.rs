use std::env;

use crate::KoraError;

pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, anyhow::Error> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub fn bytes_to_hex(bytes: &[u8]) -> Result<String, anyhow::Error> {
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn get_env_var_for_signer(env_var_name: &str, signer_name: &str) -> Result<String, KoraError> {
    env::var(env_var_name).map_err(|_| {
        KoraError::ValidationError(format!(
            "Environment variable '{env_var_name}' required for signer '{signer_name}' is not set"
        ))
    })
}
