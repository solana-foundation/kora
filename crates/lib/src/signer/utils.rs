use std::env;

use crate::KoraError;

pub fn get_env_var_for_signer(env_var_name: &str, signer_name: &str) -> Result<String, KoraError> {
    env::var(env_var_name).map_err(|_| {
        KoraError::ValidationError(format!(
            "Environment variable '{env_var_name}' required for signer '{signer_name}' is not set"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;

    #[test]
    fn test_get_env_var_for_signer_exists() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::set_var("TEST_SIGNER_VAR", "test_value");
        let result = get_env_var_for_signer("TEST_SIGNER_VAR", "test_signer").unwrap();
        assert_eq!(result, "test_value");
        env::remove_var("TEST_SIGNER_VAR");
    }

    #[test]
    fn test_get_env_var_for_signer_not_exists() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::remove_var("NONEXISTENT_VAR");
        let result = get_env_var_for_signer("NONEXISTENT_VAR", "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }
}
