use crate::{error::KoraError, sanitize_error, signer::utils::get_env_var_for_signer};
use serde::{Deserialize, Serialize};
use solana_signers::Signer;
use std::{fmt, fs, path::Path};

/// Configuration for a pool of signers
#[derive(Clone, Serialize, Deserialize)]
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

impl fmt::Display for SelectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SelectionStrategy::RoundRobin => "round_robin",
            SelectionStrategy::Random => "random",
            SelectionStrategy::Weighted => "weighted",
        };
        write!(f, "{s}")
    }
}

fn default_strategy() -> SelectionStrategy {
    SelectionStrategy::RoundRobin
}

/// Configuration for an individual signer
#[derive(Clone, Serialize, Deserialize)]
pub struct SignerConfig {
    /// Human-readable name for this signer
    pub name: String,
    /// Weight for weighted selection strategy (optional, defaults to 1)
    pub weight: Option<u32>,

    /// Signer-specific configuration
    #[serde(flatten)]
    pub config: SignerTypeConfig,
}

/// Memory signer configuration (local keypair)
#[derive(Clone, Serialize, Deserialize)]
pub struct MemorySignerConfig {
    pub private_key_env: String,
}

/// Turnkey signer configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct TurnkeySignerConfig {
    pub api_public_key_env: String,
    pub api_private_key_env: String,
    pub organization_id_env: String,
    pub private_key_id_env: String,
    pub public_key_env: String,
}

/// Privy signer configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct PrivySignerConfig {
    pub app_id_env: String,
    pub app_secret_env: String,
    pub wallet_id_env: String,
}

/// Vault signer configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct VaultSignerConfig {
    pub vault_addr_env: String,
    pub vault_token_env: String,
    pub key_name_env: String,
    pub pubkey_env: String,
}

/// Signer type-specific configuration
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignerTypeConfig {
    /// Memory signer configuration
    Memory {
        #[serde(flatten)]
        config: MemorySignerConfig,
    },
    /// Turnkey signer configuration
    Turnkey {
        #[serde(flatten)]
        config: TurnkeySignerConfig,
    },
    /// Privy signer configuration
    Privy {
        #[serde(flatten)]
        config: PrivySignerConfig,
    },
    /// Vault signer configuration
    Vault {
        #[serde(flatten)]
        config: VaultSignerConfig,
    },
}

impl SignerPoolConfig {
    /// Load signer pool configuration from TOML file
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, KoraError> {
        let contents = fs::read_to_string(path).map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to read signer config file: {}",
                sanitize_error!(e)
            ))
        })?;

        let config: SignerPoolConfig = toml::from_str(&contents).map_err(|e| {
            KoraError::ValidationError(format!(
                "Failed to parse signers config TOML: {}",
                sanitize_error!(e)
            ))
        })?;

        config.validate_signer_config()?;

        Ok(config)
    }

    /// Validate the signer pool configuration
    pub fn validate_signer_config(&self) -> Result<(), KoraError> {
        self.validate_signer_not_empty()?;

        for (index, signer) in self.signers.iter().enumerate() {
            signer.validate_individual_signer_config(index)?;
        }

        self.validate_signer_names()?;
        self.validate_strategy_weights()?;

        Ok(())
    }

    pub fn validate_signer_not_empty(&self) -> Result<(), KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::ValidationError(
                "At least one signer must be configured".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_signer_names(&self) -> Result<(), KoraError> {
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

    pub fn validate_strategy_weights(&self) -> Result<(), KoraError> {
        if matches!(self.signer_pool.strategy, SelectionStrategy::Weighted) {
            for signer in &self.signers {
                if let Some(weight) = signer.weight {
                    if weight == 0 {
                        return Err(KoraError::ValidationError(format!(
                            "Signer '{}' has weight of 0 in weighted strategy",
                            signer.name
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

impl SignerConfig {
    /// Build an external signer from configuration by resolving environment variables
    pub async fn build_signer_from_config(config: &SignerConfig) -> Result<Signer, KoraError> {
        match &config.config {
            SignerTypeConfig::Memory { config: memory_config } => {
                Self::build_memory_signer(memory_config, &config.name)
            }
            SignerTypeConfig::Turnkey { config: turnkey_config } => {
                Self::build_turnkey_signer(turnkey_config, &config.name)
            }
            SignerTypeConfig::Privy { config: privy_config } => {
                Self::build_privy_signer(privy_config, &config.name).await
            }
            SignerTypeConfig::Vault { config: vault_config } => {
                Self::build_vault_signer(vault_config, &config.name)
            }
        }
    }

    fn build_memory_signer(
        config: &MemorySignerConfig,
        signer_name: &str,
    ) -> Result<Signer, KoraError> {
        let private_key = get_env_var_for_signer(&config.private_key_env, signer_name)?;
        Signer::from_memory(&private_key).map_err(|e| {
            KoraError::SigningError(format!(
                "Failed to create memory signer '{signer_name}': {}",
                sanitize_error!(e)
            ))
        })
    }

    fn build_turnkey_signer(
        config: &TurnkeySignerConfig,
        signer_name: &str,
    ) -> Result<Signer, KoraError> {
        let api_public_key = get_env_var_for_signer(&config.api_public_key_env, signer_name)?;
        let api_private_key = get_env_var_for_signer(&config.api_private_key_env, signer_name)?;
        let organization_id = get_env_var_for_signer(&config.organization_id_env, signer_name)?;
        let private_key_id = get_env_var_for_signer(&config.private_key_id_env, signer_name)?;
        let public_key = get_env_var_for_signer(&config.public_key_env, signer_name)?;

        Signer::from_turnkey(
            api_public_key,
            api_private_key,
            organization_id,
            private_key_id,
            public_key,
        )
        .map_err(|e| {
            KoraError::SigningError(format!(
                "Failed to create Turnkey signer '{signer_name}': {}",
                sanitize_error!(e)
            ))
        })
    }

    async fn build_privy_signer(
        config: &PrivySignerConfig,
        signer_name: &str,
    ) -> Result<Signer, KoraError> {
        let app_id = get_env_var_for_signer(&config.app_id_env, signer_name)?;
        let app_secret = get_env_var_for_signer(&config.app_secret_env, signer_name)?;
        let wallet_id = get_env_var_for_signer(&config.wallet_id_env, signer_name)?;

        Signer::from_privy(app_id, app_secret, wallet_id).await.map_err(|e| {
            KoraError::SigningError(format!(
                "Failed to create Privy signer '{signer_name}': {}",
                sanitize_error!(e)
            ))
        })
    }

    fn build_vault_signer(
        config: &VaultSignerConfig,
        signer_name: &str,
    ) -> Result<Signer, KoraError> {
        let vault_addr = get_env_var_for_signer(&config.vault_addr_env, signer_name)?;
        let vault_token = get_env_var_for_signer(&config.vault_token_env, signer_name)?;
        let key_name = get_env_var_for_signer(&config.key_name_env, signer_name)?;
        let pubkey = get_env_var_for_signer(&config.pubkey_env, signer_name)?;

        Signer::from_vault(vault_addr, vault_token, key_name, pubkey).map_err(|e| {
            KoraError::SigningError(format!(
                "Failed to create Vault signer '{signer_name}': {}",
                sanitize_error!(e)
            ))
        })
    }

    /// Validate an individual signer configuration
    pub fn validate_individual_signer_config(&self, index: usize) -> Result<(), KoraError> {
        if self.name.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "Signer at index {index} must have a non-empty name"
            )));
        }

        match &self.config {
            SignerTypeConfig::Memory { config } => Self::validate_memory_config(config, &self.name),
            SignerTypeConfig::Turnkey { config } => {
                Self::validate_turnkey_config(config, &self.name)
            }
            SignerTypeConfig::Privy { config } => Self::validate_privy_config(config, &self.name),
            SignerTypeConfig::Vault { config } => Self::validate_vault_config(config, &self.name),
        }
    }

    fn validate_memory_config(
        config: &MemorySignerConfig,
        signer_name: &str,
    ) -> Result<(), KoraError> {
        if config.private_key_env.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "Memory signer '{signer_name}' must specify non-empty private_key_env"
            )));
        }
        Ok(())
    }

    fn validate_turnkey_config(
        config: &TurnkeySignerConfig,
        signer_name: &str,
    ) -> Result<(), KoraError> {
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

    fn validate_privy_config(
        config: &PrivySignerConfig,
        signer_name: &str,
    ) -> Result<(), KoraError> {
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

    fn validate_vault_config(
        config: &VaultSignerConfig,
        signer_name: &str,
    ) -> Result<(), KoraError> {
        let env_vars = [
            ("vault_addr_env", &config.vault_addr_env),
            ("vault_token_env", &config.vault_token_env),
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

        if let SignerTypeConfig::Memory { config } = &signer1.config {
            assert_eq!(config.private_key_env, "SIGNER_1_PRIVATE_KEY");
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
                    config: MemorySignerConfig { private_key_env: "TEST_PRIVATE_KEY".to_string() },
                },
            }],
        };

        assert!(config.validate_signer_config().is_ok());
        assert!(config.validate_strategy_weights().is_ok());
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
                        config: MemorySignerConfig {
                            private_key_env: "TEST_PRIVATE_KEY_1".to_string(),
                        },
                    },
                },
                SignerConfig {
                    name: "duplicate".to_string(),
                    weight: Some(1),
                    config: SignerTypeConfig::Memory {
                        config: MemorySignerConfig {
                            private_key_env: "TEST_PRIVATE_KEY_2".to_string(),
                        },
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
