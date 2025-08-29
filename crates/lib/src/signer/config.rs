use crate::{
    error::KoraError,
    signer::{
        config_trait::SignerConfigTrait,
        memory_signer::config::{MemorySignerConfig, MemorySignerHandler},
        privy::config::{PrivySignerConfig, PrivySignerHandler},
        turnkey::config::{TurnkeySignerConfig, TurnkeySignerHandler},
        vault::config::{VaultSignerConfig, VaultSignerHandler},
        KoraSigner,
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

/// Signer type-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        // Validate that at least one signer is configured
        self.validate_signer_not_empty()?;

        // Validate each signer configuration
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
    /// Build a KoraSigner from configuration by resolving environment variables
    pub async fn build_signer_from_config(config: &SignerConfig) -> Result<KoraSigner, KoraError> {
        match &config.config {
            SignerTypeConfig::Memory { config: memory_config } => {
                MemorySignerHandler::build_from_config(memory_config, &config.name)
            }
            SignerTypeConfig::Turnkey { config: turnkey_config } => {
                TurnkeySignerHandler::build_from_config(turnkey_config, &config.name)
            }
            SignerTypeConfig::Privy { config: privy_config } => {
                PrivySignerHandler::build_from_config(privy_config, &config.name)
            }
            SignerTypeConfig::Vault { config: vault_config } => {
                VaultSignerHandler::build_from_config(vault_config, &config.name)
            }
        }
    }

    /// Validate an individual signer configuration
    pub fn validate_individual_signer_config(&self, index: usize) -> Result<(), KoraError> {
        if self.name.is_empty() {
            return Err(KoraError::ValidationError(format!(
                "Signer at index {index} must have a non-empty name"
            )));
        }

        // Delegate validation to signer-specific handlers
        match &self.config {
            SignerTypeConfig::Memory { config: memory_config } => {
                MemorySignerHandler::validate_config(memory_config, &self.name)
            }
            SignerTypeConfig::Turnkey { config: turnkey_config } => {
                TurnkeySignerHandler::validate_config(turnkey_config, &self.name)
            }
            SignerTypeConfig::Privy { config: privy_config } => {
                PrivySignerHandler::validate_config(privy_config, &self.name)
            }
            SignerTypeConfig::Vault { config: vault_config } => {
                VaultSignerHandler::validate_config(vault_config, &self.name)
            }
        }
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
