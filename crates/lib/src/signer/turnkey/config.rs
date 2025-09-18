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
        );

        Ok(KoraSigner::Turnkey(signer))
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

        let config = TurnkeySignerConfig {
            api_public_key_env: "VALID_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "VALID_API_PRIVATE_KEY".to_string(),
            organization_id_env: "VALID_ORG_ID".to_string(),
            private_key_id_env: "VALID_PRIVATE_KEY_ID".to_string(),
            public_key_env: "VALID_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_empty_api_public_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = TurnkeySignerConfig {
            api_public_key_env: "".to_string(),
            api_private_key_env: "VALID_API_PRIVATE_KEY".to_string(),
            organization_id_env: "VALID_ORG_ID".to_string(),
            private_key_id_env: "VALID_PRIVATE_KEY_ID".to_string(),
            public_key_env: "VALID_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_api_private_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = TurnkeySignerConfig {
            api_public_key_env: "VALID_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "".to_string(),
            organization_id_env: "VALID_ORG_ID".to_string(),
            private_key_id_env: "VALID_PRIVATE_KEY_ID".to_string(),
            public_key_env: "VALID_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_organization_id() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = TurnkeySignerConfig {
            api_public_key_env: "VALID_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "VALID_API_PRIVATE_KEY".to_string(),
            organization_id_env: "".to_string(),
            private_key_id_env: "VALID_PRIVATE_KEY_ID".to_string(),
            public_key_env: "VALID_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_private_key_id() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = TurnkeySignerConfig {
            api_public_key_env: "VALID_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "VALID_API_PRIVATE_KEY".to_string(),
            organization_id_env: "VALID_ORG_ID".to_string(),
            private_key_id_env: "".to_string(),
            public_key_env: "VALID_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_public_key() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = TurnkeySignerConfig {
            api_public_key_env: "VALID_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "VALID_API_PRIVATE_KEY".to_string(),
            organization_id_env: "VALID_ORG_ID".to_string(),
            private_key_id_env: "VALID_PRIVATE_KEY_ID".to_string(),
            public_key_env: "".to_string(),
        };

        let result = TurnkeySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_missing_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Ensure env vars don't exist
        env::remove_var("NONEXISTENT_API_PUBLIC_KEY");
        env::remove_var("NONEXISTENT_API_PRIVATE_KEY");
        env::remove_var("NONEXISTENT_ORG_ID");
        env::remove_var("NONEXISTENT_PRIVATE_KEY_ID");
        env::remove_var("NONEXISTENT_PUBLIC_KEY");

        let config = TurnkeySignerConfig {
            api_public_key_env: "NONEXISTENT_API_PUBLIC_KEY".to_string(),
            api_private_key_env: "NONEXISTENT_API_PRIVATE_KEY".to_string(),
            organization_id_env: "NONEXISTENT_ORG_ID".to_string(),
            private_key_id_env: "NONEXISTENT_PRIVATE_KEY_ID".to_string(),
            public_key_env: "NONEXISTENT_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_partial_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set only some environment variables
        env::set_var("TEST_TURNKEY_API_PUBLIC_KEY_PARTIAL", "test_api_public_key");
        env::set_var("TEST_TURNKEY_API_PRIVATE_KEY_PARTIAL", "test_api_private_key");
        env::remove_var("MISSING_ORG_ID");
        env::remove_var("MISSING_PRIVATE_KEY_ID");
        env::remove_var("MISSING_PUBLIC_KEY");

        let config = TurnkeySignerConfig {
            api_public_key_env: "TEST_TURNKEY_API_PUBLIC_KEY_PARTIAL".to_string(),
            api_private_key_env: "TEST_TURNKEY_API_PRIVATE_KEY_PARTIAL".to_string(),
            organization_id_env: "MISSING_ORG_ID".to_string(),
            private_key_id_env: "MISSING_PRIVATE_KEY_ID".to_string(),
            public_key_env: "MISSING_PUBLIC_KEY".to_string(),
        };

        let result = TurnkeySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));

        // Clean up
        env::remove_var("TEST_TURNKEY_API_PUBLIC_KEY_PARTIAL");
        env::remove_var("TEST_TURNKEY_API_PRIVATE_KEY_PARTIAL");
    }

    #[test]
    fn test_build_from_config_valid_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set test environment variables with valid values
        env::set_var("TEST_TURNKEY_API_PUBLIC_KEY_VALID", "test_api_public_key");
        env::set_var("TEST_TURNKEY_API_PRIVATE_KEY_VALID", "test_api_private_key");
        env::set_var("TEST_TURNKEY_ORG_ID_VALID", "test_org_id");
        env::set_var("TEST_TURNKEY_PRIVATE_KEY_ID_VALID", "test_private_key_id");
        env::set_var(
            "TEST_TURNKEY_PUBLIC_KEY_VALID",
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        );

        let config = TurnkeySignerConfig {
            api_public_key_env: "TEST_TURNKEY_API_PUBLIC_KEY_VALID".to_string(),
            api_private_key_env: "TEST_TURNKEY_API_PRIVATE_KEY_VALID".to_string(),
            organization_id_env: "TEST_TURNKEY_ORG_ID_VALID".to_string(),
            private_key_id_env: "TEST_TURNKEY_PRIVATE_KEY_ID_VALID".to_string(),
            public_key_env: "TEST_TURNKEY_PUBLIC_KEY_VALID".to_string(),
        };

        let result = TurnkeySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_ok());

        // Clean up
        env::remove_var("TEST_TURNKEY_API_PUBLIC_KEY_VALID");
        env::remove_var("TEST_TURNKEY_API_PRIVATE_KEY_VALID");
        env::remove_var("TEST_TURNKEY_ORG_ID_VALID");
        env::remove_var("TEST_TURNKEY_PRIVATE_KEY_ID_VALID");
        env::remove_var("TEST_TURNKEY_PUBLIC_KEY_VALID");
    }
}
