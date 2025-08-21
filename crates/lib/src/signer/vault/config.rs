use serde::{Deserialize, Serialize};

use crate::{
    error::KoraError,
    signer::{
        config_trait::SignerConfigTrait, utils::get_env_var_for_signer,
        vault::vault_signer::VaultSigner, KoraSigner,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSignerConfig {
    /// Environment variable for Vault server address
    pub addr_env: String,
    /// Environment variable for Vault authentication token
    pub token_env: String,
    /// Environment variable for Vault key name
    pub key_name_env: String,
    /// Environment variable for Vault public key
    pub pubkey_env: String,
}
pub struct VaultSignerHandler;

impl SignerConfigTrait for VaultSignerHandler {
    type Config = VaultSignerConfig;

    fn validate_config(config: &Self::Config, signer_name: &str) -> Result<(), KoraError> {
        let env_vars = [
            ("addr_env", &config.addr_env),
            ("token_env", &config.token_env),
            ("key_name_env", &config.key_name_env),
            ("pubkey_env", &config.pubkey_env),
        ];

        for (field_name, env_var) in env_vars {
            if env_var.is_empty() {
                return Err(KoraError::ValidationError(format!(
                    "Vault signer '{signer_name}' must specify non-empty {field_name}"
                )));
            }
        }

        Ok(())
    }

    fn build_from_config(
        config: &Self::Config,
        signer_name: &str,
    ) -> Result<KoraSigner, KoraError> {
        let addr = get_env_var_for_signer(&config.addr_env, signer_name)?;
        let token = get_env_var_for_signer(&config.token_env, signer_name)?;
        let key_name = get_env_var_for_signer(&config.key_name_env, signer_name)?;
        let pubkey = get_env_var_for_signer(&config.pubkey_env, signer_name)?;

        let signer = VaultSigner::new(addr, token, key_name, pubkey).map_err(|e| {
            KoraError::ValidationError(format!(
                "Failed to create Vault signer '{signer_name}': {e}"
            ))
        })?;

        Ok(KoraSigner::Vault(signer))
    }
}
