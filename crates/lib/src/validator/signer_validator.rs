use crate::signer::{SelectionStrategy, SignerPoolConfig};
pub struct SignerValidator {}

impl SignerValidator {
    /// Validate signer configuration with detailed results
    pub fn validate_with_result(config: &SignerPoolConfig) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if signers list is empty
        if config.signers.is_empty() {
            errors.push("No signers configured".to_string());
            return (warnings, errors);
        }

        // Validate strategy-specific requirements
        match config.signer_pool.strategy {
            SelectionStrategy::Weighted => {
                for signer in &config.signers {
                    if let Some(weight) = signer.weight {
                        if weight == 0 {
                            errors.push(format!(
                                "Signer '{}' has weight of 0 in weighted strategy",
                                signer.name
                            ));
                        }
                    } else {
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
                            match config.signer_pool.strategy {
                                SelectionStrategy::RoundRobin => "round_robin",
                                SelectionStrategy::Random => "random",
                                _ => "unknown",
                            }
                        ));
                    }
                }
            }
        }

        // Check for duplicate names
        let mut names = std::collections::HashSet::new();
        for signer in &config.signers {
            if !names.insert(&signer.name) {
                errors.push(format!("Duplicate signer name: {}", signer.name));
            }
        }

        // Validate each signer's configuration
        for (index, signer) in config.signers.iter().enumerate() {
            if signer.name.is_empty() {
                errors.push(format!("Signer at index {index} has empty name"));
            }

            // Check environment variable references
            match &signer.config {
                crate::signer::SignerTypeConfig::Memory { config } => {
                    if config.private_key_env.is_empty() {
                        errors.push(format!(
                            "Memory signer '{}' has empty private_key_env",
                            signer.name
                        ));
                    }
                }
                crate::signer::SignerTypeConfig::Turnkey { config } => {
                    if config.api_public_key_env.is_empty() {
                        errors.push(format!(
                            "Turnkey signer '{}' has empty api_public_key_env",
                            signer.name
                        ));
                    }
                    if config.api_private_key_env.is_empty() {
                        errors.push(format!(
                            "Turnkey signer '{}' has empty api_private_key_env",
                            signer.name
                        ));
                    }
                    if config.organization_id_env.is_empty() {
                        errors.push(format!(
                            "Turnkey signer '{}' has empty organization_id_env",
                            signer.name
                        ));
                    }
                    if config.private_key_id_env.is_empty() {
                        errors.push(format!(
                            "Turnkey signer '{}' has empty private_key_id_env",
                            signer.name
                        ));
                    }
                    if config.public_key_env.is_empty() {
                        errors.push(format!(
                            "Turnkey signer '{}' has empty public_key_env",
                            signer.name
                        ));
                    }
                }
                crate::signer::SignerTypeConfig::Privy { config } => {
                    if config.app_id_env.is_empty() {
                        errors.push(format!("Privy signer '{}' has empty app_id_env", signer.name));
                    }
                    if config.app_secret_env.is_empty() {
                        errors.push(format!(
                            "Privy signer '{}' has empty app_secret_env",
                            signer.name
                        ));
                    }
                    if config.wallet_id_env.is_empty() {
                        errors.push(format!(
                            "Privy signer '{}' has empty wallet_id_env",
                            signer.name
                        ));
                    }
                }
                crate::signer::SignerTypeConfig::Vault { config } => {
                    if config.addr_env.is_empty() {
                        errors.push(format!("Vault signer '{}' has empty addr_env", signer.name));
                    }
                    if config.token_env.is_empty() {
                        errors.push(format!("Vault signer '{}' has empty token_env", signer.name));
                    }
                    if config.key_name_env.is_empty() {
                        errors
                            .push(format!("Vault signer '{}' has empty key_name_env", signer.name));
                    }
                    if config.pubkey_env.is_empty() {
                        errors.push(format!("Vault signer '{}' has empty pubkey_env", signer.name));
                    }
                }
            }
        }

        (warnings, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::{
        config::SignerPoolSettings, memory_signer::config::MemorySignerConfig, SignerConfig,
        SignerTypeConfig,
    };

    #[test]
    fn test_validate_with_result_warnings() {
        let config = SignerPoolConfig {
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
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
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
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
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::Weighted },
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
            signer_pool: SignerPoolSettings { strategy: SelectionStrategy::RoundRobin },
            signers: vec![],
        };

        let (_warnings, errors) = SignerValidator::validate_with_result(&config);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("No signers configured")));
    }
}
