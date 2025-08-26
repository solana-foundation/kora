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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;
    use std::env;

    #[test]
    fn test_validate_config_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = PrivySignerConfig {
            app_id_env: "VALID_APP_ID".to_string(),
            app_secret_env: "VALID_APP_SECRET".to_string(),
            wallet_id_env: "VALID_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_empty_app_id() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = PrivySignerConfig {
            app_id_env: "".to_string(),
            app_secret_env: "VALID_APP_SECRET".to_string(),
            wallet_id_env: "VALID_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_app_secret() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = PrivySignerConfig {
            app_id_env: "VALID_APP_ID".to_string(),
            app_secret_env: "".to_string(),
            wallet_id_env: "VALID_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_validate_config_empty_wallet_id() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let config = PrivySignerConfig {
            app_id_env: "VALID_APP_ID".to_string(),
            app_secret_env: "VALID_APP_SECRET".to_string(),
            wallet_id_env: "".to_string(),
        };

        let result = PrivySignerHandler::validate_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_missing_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Ensure env vars don't exist
        env::remove_var("NONEXISTENT_APP_ID");
        env::remove_var("NONEXISTENT_APP_SECRET");
        env::remove_var("NONEXISTENT_WALLET_ID");

        let config = PrivySignerConfig {
            app_id_env: "NONEXISTENT_APP_ID".to_string(),
            app_secret_env: "NONEXISTENT_APP_SECRET".to_string(),
            wallet_id_env: "NONEXISTENT_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_build_from_config_valid_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set test environment variables
        env::set_var("TEST_PRIVY_APP_ID", "test_app_id");
        env::set_var("TEST_PRIVY_APP_SECRET", "test_app_secret");
        env::set_var("TEST_PRIVY_WALLET_ID", "test_wallet_id");

        let config = PrivySignerConfig {
            app_id_env: "TEST_PRIVY_APP_ID".to_string(),
            app_secret_env: "TEST_PRIVY_APP_SECRET".to_string(),
            wallet_id_env: "TEST_PRIVY_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), KoraSigner::Privy(_)));

        // Clean up
        env::remove_var("TEST_PRIVY_APP_ID");
        env::remove_var("TEST_PRIVY_APP_SECRET");
        env::remove_var("TEST_PRIVY_WALLET_ID");
    }

    #[test]
    fn test_build_from_config_partial_env_vars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        // Set only some environment variables
        env::set_var("TEST_PRIVY_APP_ID_PARTIAL", "test_app_id");
        env::remove_var("MISSING_APP_SECRET");
        env::remove_var("MISSING_WALLET_ID");

        let config = PrivySignerConfig {
            app_id_env: "TEST_PRIVY_APP_ID_PARTIAL".to_string(),
            app_secret_env: "MISSING_APP_SECRET".to_string(),
            wallet_id_env: "MISSING_WALLET_ID".to_string(),
        };

        let result = PrivySignerHandler::build_from_config(&config, "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));

        // Clean up
        env::remove_var("TEST_PRIVY_APP_ID_PARTIAL");
    }
}
