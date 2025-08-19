use crate::{
    error::KoraError,
    signer::{
        memory_signer::solana_signer::SolanaMemorySigner, privy::types::PrivySigner,
        turnkey::types::TurnkeySigner, utils::get_env_var_for_signer,
        vault::vault_signer::VaultSigner, KoraSigner,
    },
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Configuration for a pool of signers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerPoolConfig {
    /// Signer pool configuration
    pub signer_pool: SignerPoolSettings,
    /// List of individual signer configurations
    pub signers: Vec<SignerConfig>,
}

/// Settings for the signer pool behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerPoolSettings {
    /// Selection strategy for choosing signers
    #[serde(default = "default_strategy")]
    pub strategy: SelectionStrategy,
}

/// Available signer selection strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    RoundRobin,
    Random,
    Weighted,
}

fn default_strategy() -> SelectionStrategy {
    SelectionStrategy::RoundRobin
}

/// Configuration for an individual signer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerConfig {
    /// Human-readable name for this signer
    pub name: String,
    /// Weight for weighted selection strategy (optional, defaults to 1)
    pub weight: Option<u32>,

    /// Signer-specific configuration
    #[serde(flatten)]
    pub config: SignerTypeConfig,
}

/// Signer type-specific configuration with environment variable references
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignerTypeConfig {
    /// Memory signer configuration
    Memory {
        /// Environment variable containing the private key
        private_key_env: String,
    },
    /// Turnkey signer configuration
    Turnkey {
        /// Environment variable for Turnkey API public key
        api_public_key_env: String,
        /// Environment variable for Turnkey API private key
        api_private_key_env: String,
        /// Environment variable for Turnkey organization ID
        organization_id_env: String,
        /// Environment variable for Turnkey private key ID
        private_key_id_env: String,
        /// Environment variable for Turnkey public key
        public_key_env: String,
    },
    /// Privy signer configuration
    Privy {
        /// Environment variable for Privy app ID
        app_id_env: String,
        /// Environment variable for Privy app secret
        app_secret_env: String,
        /// Environment variable for Privy wallet ID
        wallet_id_env: String,
    },
    /// Vault signer configuration
    Vault {
        /// Environment variable for Vault server address
        addr_env: String,
        /// Environment variable for Vault authentication token
        token_env: String,
        /// Environment variable for Vault key name
        key_name_env: String,
        /// Environment variable for Vault public key
        pubkey_env: String,
    },
}

impl SignerPoolConfig {
    /// Load signer pool configuration from TOML file
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, KoraError> {
        let contents = fs::read_to_string(path).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to read config file: {e}"))
        })?;

        let config: SignerPoolConfig = toml::from_str(&contents).map_err(|e| {
            KoraError::ValidationError(format!("Failed to parse signers config TOML: {e}"))
        })?;

        config.validate_signer_config()?;

        Ok(config)
    }

    /// Validate the signer pool configuration
    pub fn validate_signer_config(&self) -> Result<(), KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::ValidationError(
                "At least one signer must be configured".to_string(),
            ));
        }

        // Validate each signer configuration
        for (index, signer) in self.signers.iter().enumerate() {
            signer.validate_individual_signer_config(index)?;
        }

        // Check for duplicate names
        let mut names = std::collections::HashSet::new();
        for signer in &self.signers {
            if !names.insert(&signer.name) {
                return Err(KoraError::ValidationError(format!(
                    "Duplicate signer name: {}",
                    signer.name
                )));
            }
        }

        Ok(())
    }
}

impl SignerConfig {
    /// Build a KoraSigner from configuration by resolving environment variables
    pub async fn build_signer_from_config(config: &SignerConfig) -> Result<KoraSigner, KoraError> {
        match &config.config {
            SignerTypeConfig::Memory { private_key_env } => {
                let private_key = get_env_var_for_signer(private_key_env, &config.name)?;
                let signer =
                    SolanaMemorySigner::from_private_key_string(&private_key).map_err(|e| {
                        KoraError::ValidationError(format!(
                            "Failed to create memory signer '{}': {}",
                            config.name, e
                        ))
                    })?;
                Ok(KoraSigner::Memory(signer))
            }
            SignerTypeConfig::Turnkey {
                api_public_key_env,
                api_private_key_env,
                organization_id_env,
                private_key_id_env,
                public_key_env,
            } => {
                let api_public_key = get_env_var_for_signer(api_public_key_env, &config.name)?;
                let api_private_key = get_env_var_for_signer(api_private_key_env, &config.name)?;
                let organization_id = get_env_var_for_signer(organization_id_env, &config.name)?;
                let private_key_id = get_env_var_for_signer(private_key_id_env, &config.name)?;
                let public_key = get_env_var_for_signer(public_key_env, &config.name)?;

                let signer = TurnkeySigner::new(
                    api_public_key,
                    api_private_key,
                    organization_id,
                    private_key_id,
                    public_key,
                )
                .map_err(|e| {
                    KoraError::ValidationError(format!(
                        "Failed to create Turnkey signer '{}': {}",
                        config.name, e
                    ))
                })?;

                Ok(KoraSigner::Turnkey(signer))
            }
            SignerTypeConfig::Privy { app_id_env, app_secret_env, wallet_id_env } => {
                let app_id = get_env_var_for_signer(app_id_env, &config.name)?;
                let app_secret = get_env_var_for_signer(app_secret_env, &config.name)?;
                let wallet_id = get_env_var_for_signer(wallet_id_env, &config.name)?;

                let signer = PrivySigner::new(app_id, app_secret, wallet_id);

                Ok(KoraSigner::Privy(signer))
            }
            SignerTypeConfig::Vault { addr_env, token_env, key_name_env, pubkey_env } => {
                let addr = get_env_var_for_signer(addr_env, &config.name)?;
                let token = get_env_var_for_signer(token_env, &config.name)?;
                let key_name = get_env_var_for_signer(key_name_env, &config.name)?;
                let pubkey = get_env_var_for_signer(pubkey_env, &config.name)?;

                let signer = VaultSigner::new(addr, token, key_name, pubkey).map_err(|e| {
                    KoraError::ValidationError(format!(
                        "Failed to create Vault signer '{}': {}",
                        config.name, e
                    ))
                })?;

                Ok(KoraSigner::Vault(signer))
            }
        }
    }

    /// Validate an individual signer configuration
    fn validate_individual_signer_config(&self, index: usize) -> Result<(), KoraError> {
        if self.name.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "Signer at index {index} must have a non-empty name"
            )));
        }

        // Validate fields based on signer type
        match &self.config {
            SignerTypeConfig::Memory { private_key_env } => {
                if private_key_env.is_empty() {
                    return Err(KoraError::ValidationError(format!(
                        "Memory signer '{}' must specify non-empty private_key_env",
                        self.name
                    )));
                }
            }
            SignerTypeConfig::Turnkey {
                api_public_key_env,
                api_private_key_env,
                organization_id_env,
                private_key_id_env,
                public_key_env,
            } => {
                let env_vars = [
                    ("api_public_key_env", api_public_key_env),
                    ("api_private_key_env", api_private_key_env),
                    ("organization_id_env", organization_id_env),
                    ("private_key_id_env", private_key_id_env),
                    ("public_key_env", public_key_env),
                ];
                for (field_name, env_var) in env_vars {
                    if env_var.is_empty() {
                        return Err(KoraError::ValidationError(format!(
                            "Turnkey signer '{}' must specify non-empty {}",
                            self.name, field_name
                        )));
                    }
                }
            }
            SignerTypeConfig::Privy { app_id_env, app_secret_env, wallet_id_env } => {
                let env_vars = [
                    ("app_id_env", app_id_env),
                    ("app_secret_env", app_secret_env),
                    ("wallet_id_env", wallet_id_env),
                ];
                for (field_name, env_var) in env_vars {
                    if env_var.is_empty() {
                        return Err(KoraError::ValidationError(format!(
                            "Privy signer '{}' must specify non-empty {}",
                            self.name, field_name
                        )));
                    }
                }
            }
            SignerTypeConfig::Vault { addr_env, token_env, key_name_env, pubkey_env } => {
                let env_vars = [
                    ("addr_env", addr_env),
                    ("token_env", token_env),
                    ("key_name_env", key_name_env),
                    ("pubkey_env", pubkey_env),
                ];
                for (field_name, env_var) in env_vars {
                    if env_var.is_empty() {
                        return Err(KoraError::ValidationError(format!(
                            "Vault signer '{}' must specify non-empty {}",
                            self.name, field_name
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_config() {
        let toml_content = r#"
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "memory_signer_1"
type = "memory"
private_key_env = "SIGNER_1_PRIVATE_KEY"
weight = 1

[[signers]]
name = "turnkey_signer_1" 
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY_1"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY_1"
organization_id_env = "TURNKEY_ORG_ID_1"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID_1"
public_key_env = "TURNKEY_PUBLIC_KEY_1"
weight = 2
"#;

        let config: SignerPoolConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.signers.len(), 2);
        assert!(matches!(config.signer_pool.strategy, SelectionStrategy::RoundRobin));

        // Check first signer
        let signer1 = &config.signers[0];
        assert_eq!(signer1.name, "memory_signer_1");
        assert_eq!(signer1.weight, Some(1));

        if let SignerTypeConfig::Memory { private_key_env } = &signer1.config {
            assert_eq!(private_key_env, "SIGNER_1_PRIVATE_KEY");
        } else {
            panic!("Expected Memory signer config");
        }
    }

    #[test]
    fn test_validate_config_success() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
            signers: vec![SignerConfig {
                name: "test_signer".to_string(),
                weight: Some(1),
                config: SignerTypeConfig::Memory {
                    private_key_env: "TEST_PRIVATE_KEY".to_string(),
                },
            }],
        };

        assert!(config.validate_signer_config().is_ok());
    }

    #[test]
    fn test_validate_config_empty_signers() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
            signers: vec![],
        };

        assert!(config.validate_signer_config().is_err());
    }

    #[test]
    fn test_validate_config_duplicate_names() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
            signers: vec![
                SignerConfig {
                    name: "duplicate".to_string(),
                    weight: Some(1),
                    config: SignerTypeConfig::Memory {
                        private_key_env: "TEST_PRIVATE_KEY_1".to_string(),
                    },
                },
                SignerConfig {
                    name: "duplicate".to_string(),
                    weight: Some(1),
                    config: SignerTypeConfig::Memory {
                        private_key_env: "TEST_PRIVATE_KEY_2".to_string(),
                    },
                },
            ],
        };

        assert!(config.validate_signer_config().is_err());
    }

    #[test]
    fn test_load_signers_config() {
        let toml_content = r#"
[signer_pool]
strategy = "round_robin"

[[signers]]
name = "test_signer"
type = "memory"
private_key_env = "TEST_PRIVATE_KEY"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = SignerPoolConfig::load_config(temp_file.path()).unwrap();
        assert_eq!(config.signers.len(), 1);
        assert_eq!(config.signers[0].name, "test_signer");
    }
}
