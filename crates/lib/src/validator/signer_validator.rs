use crate::{
    error::KoraError,
    signer::{SelectionStrategy, SignerPoolConfig},
};

pub struct SignerValidator {}

impl SignerValidator {
    /// Validate signer configuration with detailed results
    pub fn validate_with_result(config: &SignerPoolConfig) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if signers list is empty
        Self::try_result(config.validate_signer_not_empty(), &mut errors);

        // Validate each signer configuration - delegate to existing method
        for (index, signer) in config.signers.iter().enumerate() {
            Self::try_result(signer.validate_individual_signer_config(index), &mut errors);
        }

        // Check for duplicate names - delegate to existing method
        Self::try_result(config.validate_signer_names(), &mut errors);

        // Validate strategy weights - delegate to existing method
        Self::try_result(config.validate_strategy_weights(), &mut errors);

        // Generate strategy-specific warnings
        Self::validate_strategy_warnings(config, &mut warnings);

        (warnings, errors)
    }

    /// Helper method to convert Result to error string and add to errors vec
    fn try_result(result: Result<(), KoraError>, errors: &mut Vec<String>) {
        if let Err(KoraError::ValidationError(msg)) = result {
            errors.push(msg);
        }
    }

    /// Generate strategy-specific warnings (warnings don't fail fast)
    fn validate_strategy_warnings(config: &SignerPoolConfig, warnings: &mut Vec<String>) {
        match config.signer_pool.strategy {
            SelectionStrategy::Weighted => {
                for signer in &config.signers {
                    if signer.weight.is_none() {
                        warnings.push(format!(
                            "Signer '{}' has no weight specified for weighted strategy",
                            signer.name
                        ));
                    }
                }
            }
            _ => {
                // For non-weighted strategies, warn if weights are specified
                for signer in &config.signers {
                    if signer.weight.is_some() {
                        warnings.push(format!(
                            "Signer '{}' has weight specified but using {} strategy - weight will be ignored",
                            signer.name,
                            config.signer_pool.strategy
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::config::{
        MemorySignerConfig, SignerConfig, SignerPoolSettings, SignerTypeConfig,
    };

    #[test]
    fn test_validate_with_result_warnings() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings {
                strategy: SelectionStrategy::RoundRobin,
                failover_enabled: false,
            },
            signers: vec![SignerConfig {
                name: "test_signer".to_string(),
                weight: Some(10), // Weight specified for non-weighted strategy
                config: SignerTypeConfig::Memory {
                    config: MemorySignerConfig { private_key_env: "TEST_KEY".to_string() },
                },
            }],
        };

        let (warnings, errors) = SignerValidator::validate_with_result(&config);
        assert!(errors.is_empty());
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("weight will be ignored"));
    }

    #[test]
    fn test_validate_duplicate_names() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings {
                strategy: SelectionStrategy::RoundRobin,
                failover_enabled: false,
            },
            signers: vec![
                SignerConfig {
                    name: "duplicate".to_string(),
                    weight: None,
                    config: SignerTypeConfig::Memory {
                        config: MemorySignerConfig { private_key_env: "TEST_KEY_1".to_string() },
                    },
                },
                SignerConfig {
                    name: "duplicate".to_string(),
                    weight: None,
                    config: SignerTypeConfig::Memory {
                        config: MemorySignerConfig { private_key_env: "TEST_KEY_2".to_string() },
                    },
                },
            ],
        };

        let (_warnings, errors) = SignerValidator::validate_with_result(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("Duplicate signer name")));
    }

    #[test]
    fn test_validate_with_result_zero_weight() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings {
                strategy: SelectionStrategy::Weighted,
                failover_enabled: false,
            },
            signers: vec![SignerConfig {
                name: "test_signer".to_string(),
                weight: Some(0),
                config: SignerTypeConfig::Memory {
                    config: MemorySignerConfig { private_key_env: "TEST_KEY".to_string() },
                },
            }],
        };

        let (_warnings, errors) = SignerValidator::validate_with_result(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("weight of 0 in weighted strategy")));
    }

    #[test]
    fn test_validate_with_result_empty_signers() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings {
                strategy: SelectionStrategy::RoundRobin,
                failover_enabled: false,
            },
            signers: vec![],
        };

        let (_warnings, errors) = SignerValidator::validate_with_result(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("At least one signer must be configured")));
    }
}
