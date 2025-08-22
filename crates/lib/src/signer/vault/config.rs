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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;
    use std::env;

    #[test]
    fn test_validate_config_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = VaultSignerConfig {
            addr_env: "VALID_VAULT_ADDR".to_string(),
            token_env: "VALID_VAULT_TOKEN".to_string(),
            key_name_env: "VALID_KEY_NAME".to_string(),
            pubkey_env: "VALID_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_empty_addr() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = VaultSignerConfig {
            addr_env: "".to_string(),
            token_env: "VALID_VAULT_TOKEN".to_string(),
            key_name_env: "VALID_KEY_NAME".to_string(),
            pubkey_env: "VALID_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_token() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = VaultSignerConfig {
            addr_env: "VALID_VAULT_ADDR".to_string(),
            token_env: "".to_string(),
            key_name_env: "VALID_KEY_NAME".to_string(),
            pubkey_env: "VALID_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_key_name() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = VaultSignerConfig {
            addr_env: "VALID_VAULT_ADDR".to_string(),
            token_env: "VALID_VAULT_TOKEN".to_string(),
            key_name_env: "".to_string(),
            pubkey_env: "VALID_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_pubkey() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = VaultSignerConfig {
            addr_env: "VALID_VAULT_ADDR".to_string(),
            token_env: "VALID_VAULT_TOKEN".to_string(),
            key_name_env: "VALID_KEY_NAME".to_string(),
            pubkey_env: "".to_string(),
        };

        let result = VaultSignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_missing_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Ensure env vars don't exist
        env::remove_var("NONEXISTENT_VAULT_ADDR");
        env::remove_var("NONEXISTENT_VAULT_TOKEN");
        env::remove_var("NONEXISTENT_KEY_NAME");
        env::remove_var("NONEXISTENT_PUBKEY");

        let config = VaultSignerConfig {
            addr_env: "NONEXISTENT_VAULT_ADDR".to_string(),
            token_env: "NONEXISTENT_VAULT_TOKEN".to_string(),
            key_name_env: "NONEXISTENT_KEY_NAME".to_string(),
            pubkey_env: "NONEXISTENT_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_partial_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set only some environment variables
        env::set_var("TEST_VAULT_ADDR_PARTIAL", "https://vault.example.com");
        env::set_var("TEST_VAULT_TOKEN_PARTIAL", "test_token");
        env::remove_var("MISSING_KEY_NAME");
        env::remove_var("MISSING_PUBKEY");

        let config = VaultSignerConfig {
            addr_env: "TEST_VAULT_ADDR_PARTIAL".to_string(),
            token_env: "TEST_VAULT_TOKEN_PARTIAL".to_string(),
            key_name_env: "MISSING_KEY_NAME".to_string(),
            pubkey_env: "MISSING_PUBKEY".to_string(),
        };

        let result = VaultSignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));

        // Clean up
        env::remove_var("TEST_VAULT_ADDR_PARTIAL");
        env::remove_var("TEST_VAULT_TOKEN_PARTIAL");
    }

    #[test]
    fn test_build_from_config_valid_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set test environment variables with valid values
        env::set_var("TEST_VAULT_ADDR_VALID", "https://vault.example.com");
        env::set_var("TEST_VAULT_TOKEN_VALID", "test_token");
        env::set_var("TEST_VAULT_KEY_NAME_VALID", "test_key");
        env::set_var("TEST_VAULT_PUBKEY_VALID", "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");

        let config = VaultSignerConfig {
            addr_env: "TEST_VAULT_ADDR_VALID".to_string(),
            token_env: "TEST_VAULT_TOKEN_VALID".to_string(),
            key_name_env: "TEST_VAULT_KEY_NAME_VALID".to_string(),
            pubkey_env: "TEST_VAULT_PUBKEY_VALID".to_string(),
        };

        let result = VaultSignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_ok());

        // Clean up
        env::remove_var("TEST_VAULT_ADDR_VALID");
        env::remove_var("TEST_VAULT_TOKEN_VALID");
        env::remove_var("TEST_VAULT_KEY_NAME_VALID");
        env::remove_var("TEST_VAULT_PUBKEY_VALID");
    }
}
