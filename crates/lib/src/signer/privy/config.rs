use serde::{Deserialize, Serialize};

use crate::{
    error::KoraError,
    signer::{
        config_trait::SignerConfigTrait, privy::types::PrivySigner, utils::get_env_var_for_signer,
        KoraSigner,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivySignerConfig {
    /// Environment variable for Privy app ID
    pub app_id_env: String,
    /// Environment variable for Privy app secret
    pub app_secret_env: String,
    /// Environment variable for Privy wallet ID
    pub wallet_id_env: String,
}

pub struct PrivySignerHandler;

impl SignerConfigTrait for PrivySignerHandler {
    type Config = PrivySignerConfig;

    fn validate_config(config: &Self::Config, signer_name: &str) -> Result<(), KoraError> {
        let env_vars = [
            ("app_id_env", &config.app_id_env),
            ("app_secret_env", &config.app_secret_env),
            ("wallet_id_env", &config.wallet_id_env),
        ];

        for (field_name, env_var) in env_vars {
            if env_var.is_empty() {
                return Err(KoraError::ValidationError(format!(
                    "Privy signer '{signer_name}' must specify non-empty {field_name}"
                )));
            }
        }

        Ok(())
    }

    fn build_from_config(
        config: &Self::Config,
        signer_name: &str,
    ) -> Result<KoraSigner, KoraError> {
        let app_id = get_env_var_for_signer(&config.app_id_env, signer_name)?;
        let app_secret = get_env_var_for_signer(&config.app_secret_env, signer_name)?;
        let wallet_id = get_env_var_for_signer(&config.wallet_id_env, signer_name)?;

        let signer = PrivySigner::new(app_id, app_secret, wallet_id);

        Ok(KoraSigner::Privy(signer))
    }
}
