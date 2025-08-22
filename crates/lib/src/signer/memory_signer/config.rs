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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;
    use std::env;

    #[test]
    fn test_validate_config_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = MemorySignerConfig { private_key_env: "VALID_ENV_VAR".to_string() };

        let result = MemorySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_empty_env_var() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = MemorySignerConfig { private_key_env: "".to_string() };

        let result = MemorySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_missing_env_var() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::remove_var("NONEXISTENT_ENV_VAR");
        let config = MemorySignerConfig { private_key_env: "NONEXISTENT_ENV_VAR".to_string() };

        let result = MemorySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_invalid_private_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::set_var("INVALID_KEY_ENV", "not_a_valid_key");
        let config = MemorySignerConfig { private_key_env: "INVALID_KEY_ENV".to_string() };

        let result = MemorySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));

        env::remove_var("INVALID_KEY_ENV");
    }

    #[test]
    fn test_build_from_config_valid_private_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Use a valid base58-encoded private key for testing
        let test_private_key = "5MaiiCavjCmn9Hs1o3eznqDEhRwxo7pXiAYez7keQUviUkauRiTMD8DrESdrNjN8zd9mTmVhRvBJeg5vhyvgrAhG";
        env::set_var("VALID_PRIVATE_KEY_ENV", test_private_key);

        let config = MemorySignerConfig { private_key_env: "VALID_PRIVATE_KEY_ENV".to_string() };

        let result = MemorySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), KoraSigner::Memory(_)));

        env::remove_var("VALID_PRIVATE_KEY_ENV");
    }
}
