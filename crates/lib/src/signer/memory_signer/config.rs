use serde::{Deserialize, Serialize};

use crate::{
    error::KoraError,
    signer::{
        config_trait::SignerConfigTrait, memory_signer::solana_signer::SolanaMemorySigner,
        utils::get_env_var_for_signer, KoraSigner,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySignerConfig {
    pub private_key_env: String,
}

/// Handler for memory signer configuration
pub struct MemorySignerHandler;

impl SignerConfigTrait for MemorySignerHandler {
    type Config = MemorySignerConfig;

    fn validate_config(config: &Self::Config, signer_name: &str) -> Result<(), KoraError> {
        if config.private_key_env.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "Memory signer '{signer_name}' must specify non-empty private_key_env"
            )));
        }
        Ok(())
    }

    fn build_from_config(
        config: &Self::Config,
        signer_name: &str,
    ) -> Result<KoraSigner, KoraError> {
        let private_key = get_env_var_for_signer(&config.private_key_env, signer_name)?;
        let signer = SolanaMemorySigner::from_private_key_string(&private_key).map_err(|e| {
            KoraError::ValidationError(format!(
                "Failed to create memory signer '{signer_name}': {e}"
            ))
        })?;
        Ok(KoraSigner::Memory(signer))
    }
}
