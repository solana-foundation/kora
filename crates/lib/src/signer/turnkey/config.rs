use serde::{Deserialize, Serialize};

use crate::{
    error::KoraError,
    signer::{
        config_trait::SignerConfigTrait, turnkey::types::TurnkeySigner,
        utils::get_env_var_for_signer, KoraSigner,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnkeySignerConfig {
    /// Environment variable for Turnkey API public key
    pub api_public_key_env: String,
    /// Environment variable for Turnkey API private key
    pub api_private_key_env: String,
    /// Environment variable for Turnkey organization ID
    pub organization_id_env: String,
    /// Environment variable for Turnkey private key ID
    pub private_key_id_env: String,
    /// Environment variable for Turnkey public key
    pub public_key_env: String,
}

pub struct TurnkeySignerHandler;

impl SignerConfigTrait for TurnkeySignerHandler {
    type Config = TurnkeySignerConfig;

    fn validate_config(config: &Self::Config, signer_name: &str) -> Result<(), KoraError> {
        let env_vars = [
            ("api_public_key_env", &config.api_public_key_env),
            ("api_private_key_env", &config.api_private_key_env),
            ("organization_id_env", &config.organization_id_env),
            ("private_key_id_env", &config.private_key_id_env),
            ("public_key_env", &config.public_key_env),
        ];

        for (field_name, env_var) in env_vars {
            if env_var.is_empty() {
                return Err(KoraError::ValidationError(format!(
                    "Turnkey signer '{signer_name}' must specify non-empty {field_name}"
                )));
            }
        }

        Ok(())
    }

    fn build_from_config(
        config: &Self::Config,
        signer_name: &str,
    ) -> Result<KoraSigner, KoraError> {
        let api_public_key = get_env_var_for_signer(&config.api_public_key_env, signer_name)?;
        let api_private_key = get_env_var_for_signer(&config.api_private_key_env, signer_name)?;
        let organization_id = get_env_var_for_signer(&config.organization_id_env, signer_name)?;
        let private_key_id = get_env_var_for_signer(&config.private_key_id_env, signer_name)?;
        let public_key = get_env_var_for_signer(&config.public_key_env, signer_name)?;

        let signer = TurnkeySigner::new(
            api_public_key,
            api_private_key,
            organization_id,
            private_key_id,
            public_key,
        )
        .map_err(|e| {
            KoraError::ValidationError(format!(
                "Failed to create Turnkey signer '{signer_name}': {e}"
            ))
        })?;

        Ok(KoraSigner::Turnkey(signer))
    }
}
