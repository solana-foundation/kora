use solana_sdk::pubkey::Pubkey;

use crate::signer::privy::types::{PrivyConfig, PrivyError, PrivySigner};

impl PrivyConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            app_id: std::env::var("PRIVY_APP_ID").ok(),
            app_secret: std::env::var("PRIVY_APP_SECRET").ok(),
            wallet_id: std::env::var("PRIVY_WALLET_ID").ok(),
        }
    }

    /// Merge CLI arguments with existing config (CLI takes precedence)
    pub fn merge_with_cli(
        mut self,
        app_id: Option<String>,
        app_secret: Option<String>,
        wallet_id: Option<String>,
    ) -> Self {
        if app_id.is_some() {
            self.app_id = app_id;
        }
        if app_secret.is_some() {
            self.app_secret = app_secret;
        }
        if wallet_id.is_some() {
            self.wallet_id = wallet_id;
        }
        self
    }

    /// Build a PrivySigner from the config
    pub fn build(self) -> Result<PrivySigner, PrivyError> {
        Ok(PrivySigner {
            app_id: self.app_id.ok_or(PrivyError::MissingConfig("app_id"))?,
            app_secret: self.app_secret.ok_or(PrivyError::MissingConfig("app_secret"))?,
            wallet_id: self.wallet_id.ok_or(PrivyError::MissingConfig("wallet_id"))?,
            api_base_url: "https://api.privy.io/v1".to_string(),
            client: reqwest::Client::new(),
            public_key: Pubkey::default(), // Will be populated by init()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Save existing env vars
        let saved_app_id = std::env::var("PRIVY_APP_ID").ok();
        let saved_app_secret = std::env::var("PRIVY_APP_SECRET").ok();
        let saved_wallet_id = std::env::var("PRIVY_WALLET_ID").ok();

        // Set test env vars
        std::env::set_var("PRIVY_APP_ID", "test_app_id");
        std::env::set_var("PRIVY_APP_SECRET", "test_secret");
        std::env::set_var("PRIVY_WALLET_ID", "test_wallet");

        let config = PrivyConfig::from_env();
        assert_eq!(config.app_id, Some("test_app_id".to_string()));
        assert_eq!(config.app_secret, Some("test_secret".to_string()));
        assert_eq!(config.wallet_id, Some("test_wallet".to_string()));

        // Restore original env vars
        match saved_app_id {
            Some(val) => std::env::set_var("PRIVY_APP_ID", val),
            None => std::env::remove_var("PRIVY_APP_ID"),
        }
        match saved_app_secret {
            Some(val) => std::env::set_var("PRIVY_APP_SECRET", val),
            None => std::env::remove_var("PRIVY_APP_SECRET"),
        }
        match saved_wallet_id {
            Some(val) => std::env::set_var("PRIVY_WALLET_ID", val),
            None => std::env::remove_var("PRIVY_WALLET_ID"),
        }
    }

    #[test]
    fn test_config_merge() {
        let config = PrivyConfig {
            app_id: Some("env_id".to_string()),
            app_secret: Some("env_secret".to_string()),
            wallet_id: None,
        };

        let merged =
            config.merge_with_cli(Some("cli_id".to_string()), None, Some("cli_wallet".to_string()));

        assert_eq!(merged.app_id, Some("cli_id".to_string()));
        assert_eq!(merged.app_secret, Some("env_secret".to_string()));
        assert_eq!(merged.wallet_id, Some("cli_wallet".to_string()));
    }
}
