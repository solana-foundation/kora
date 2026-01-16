use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use utoipa::ToSchema;

use crate::{constant::DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE, error::KoraError};

use super::rules::{InstructionRule, TransactionRule, UsageRule};

/// Unified usage limit configuration
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageLimitConfig {
    /// Enable per-wallet usage limiting
    pub enabled: bool,
    /// Cache URL for shared usage limiting across multiple Kora instances (e.g., "redis://localhost:6379")
    pub cache_url: Option<String>,
    /// Fallback behavior when cache is unavailable - if true, allow transactions; if false, deny
    pub fallback_if_unavailable: bool,
    /// Usage limit rules - can be transaction-level or instruction-level
    #[serde(default)]
    pub rules: Vec<UsageLimitRuleConfig>,
}

impl Default for UsageLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_url: None,
            fallback_if_unavailable: DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE,
            rules: vec![],
        }
    }
}

impl UsageLimitConfig {
    /// Convert config rules to usage rule enums
    pub fn build_rules(&self) -> Result<Vec<UsageRule>, KoraError> {
        self.rules.iter().map(|r| r.build()).collect()
    }
}

/// Configuration for a single usage limit rule (TOML-serializable)
///
/// Use `type` field to specify the rule type:
/// - `type = "transaction"` - Counts all transactions
/// - `type = "instruction"` - Counts specific instruction types
///
/// Example TOML:
/// ```toml
/// [[kora.usage_limit.rules]]
/// type = "transaction"
/// max = 100
/// window_seconds = 3600
///
/// [[kora.usage_limit.rules]]
/// type = "instruction"
/// program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
/// instruction = "Transfer"
/// max = 10
/// ```
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UsageLimitRuleConfig {
    /// Transaction-level limit - counts all transactions for a wallet
    Transaction {
        /// Maximum transactions allowed
        max: u64,
        /// Time window in seconds (None = lifetime)
        #[serde(default)]
        window_seconds: Option<u64>,
    },
    /// Instruction-level limit - counts specific instruction types
    Instruction {
        /// Program ID (e.g., "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        program: String,
        /// Instruction type (e.g., "Transfer", "Burn")
        instruction: String,
        /// Maximum allowed
        max: u64,
        /// Time window in seconds (None = lifetime)
        #[serde(default)]
        window_seconds: Option<u64>,
    },
}

impl UsageLimitRuleConfig {
    /// Build a usage rule enum from this config
    pub fn build(&self) -> Result<UsageRule, KoraError> {
        match self {
            UsageLimitRuleConfig::Transaction { max, window_seconds } => {
                Ok(UsageRule::Transaction(TransactionRule::new(*max, *window_seconds)))
            }
            UsageLimitRuleConfig::Instruction { program, instruction, max, window_seconds } => {
                let program_pubkey = Pubkey::from_str(program).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid program in usage limit rule '{}': {}",
                        program, e
                    ))
                })?;
                Ok(UsageRule::Instruction(InstructionRule::new(
                    program_pubkey,
                    instruction.clone(),
                    *max,
                    *window_seconds,
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::toml_mock::ConfigBuilder;

    #[test]
    fn test_usage_limit_config_parsing() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Transaction lifetime rule
                (None, None, 100, None),
            ])
            .build_config()
            .unwrap();

        assert!(config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.cache_url, Some("redis://localhost:6379".to_string()));
        assert!(!config.kora.usage_limit.fallback_if_unavailable);
        assert_eq!(config.kora.usage_limit.rules.len(), 1);
        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Transaction { max, window_seconds } => {
                assert_eq!(*max, 100);
                assert_eq!(*window_seconds, None);
            }
            _ => panic!("Expected Transaction rule"),
        }
    }

    #[test]
    fn test_usage_limit_config_default() {
        let config = ConfigBuilder::new().build_config().unwrap();

        assert!(!config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.cache_url, None);
        assert!(config.kora.usage_limit.rules.is_empty());
        assert_eq!(
            config.kora.usage_limit.fallback_if_unavailable,
            DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE
        );
    }

    #[test]
    fn test_usage_limit_time_bucket_rule() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Transaction time-bucket rule: 50 per hour
                (None, None, 50, Some(3600)),
            ])
            .build_config()
            .unwrap();

        assert!(config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.rules.len(), 1);
        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Transaction { max, window_seconds } => {
                assert_eq!(*max, 50);
                assert_eq!(*window_seconds, Some(3600));
            }
            _ => panic!("Expected Transaction rule"),
        }
    }

    #[test]
    fn test_usage_limit_instruction_rules() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Time-windowed instruction rule
                (
                    Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    Some("Transfer"),
                    10,
                    Some(86400),
                ),
                // Lifetime instruction rule (no window)
                (
                    Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
                    Some("CreateIdempotent"),
                    3,
                    None,
                ),
            ])
            .build_config()
            .unwrap();

        assert!(config.kora.usage_limit.enabled);
        assert_eq!(config.kora.usage_limit.rules.len(), 2);

        // Check time-windowed rule
        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Instruction { program, instruction, max, window_seconds } => {
                assert_eq!(program, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                assert_eq!(instruction, "Transfer");
                assert_eq!(*max, 10);
                assert_eq!(*window_seconds, Some(86400));
            }
            _ => panic!("Expected Instruction rule"),
        }

        // Check lifetime rule
        match &config.kora.usage_limit.rules[1] {
            UsageLimitRuleConfig::Instruction { program, instruction, max, window_seconds } => {
                assert_eq!(program, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
                assert_eq!(instruction, "CreateIdempotent");
                assert_eq!(*max, 3);
                assert_eq!(*window_seconds, None);
            }
            _ => panic!("Expected Instruction rule"),
        }
    }

    #[test]
    fn test_transaction_rule_config_parse() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Transaction rule with window
                (None, None, 100, Some(3600)),
            ])
            .build_config()
            .unwrap();

        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Transaction { max, window_seconds } => {
                assert_eq!(*max, 100);
                assert_eq!(*window_seconds, Some(3600));
            }
            _ => panic!("Expected Transaction rule"),
        }
    }

    #[test]
    fn test_transaction_rule_config_lifetime() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Transaction lifetime rule
                (None, None, 50, None),
            ])
            .build_config()
            .unwrap();

        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Transaction { max, window_seconds } => {
                assert_eq!(*max, 50);
                assert_eq!(*window_seconds, None);
            }
            _ => panic!("Expected Transaction rule"),
        }
    }

    #[test]
    fn test_instruction_rule_config_parse() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Instruction rule with window
                (
                    Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    Some("Transfer"),
                    10,
                    Some(86400),
                ),
            ])
            .build_config()
            .unwrap();

        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Instruction { program, instruction, max, window_seconds } => {
                assert_eq!(program, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                assert_eq!(instruction, "Transfer");
                assert_eq!(*max, 10);
                assert_eq!(*window_seconds, Some(86400));
            }
            _ => panic!("Expected Instruction rule"),
        }
    }

    #[test]
    fn test_instruction_rule_config_lifetime() {
        let config = ConfigBuilder::new()
            .with_usage_limit_config(true, Some("redis://localhost:6379"), false)
            .with_usage_limit_rules(vec![
                // Instruction lifetime rule
                (Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"), Some("Burn"), 5, None),
            ])
            .build_config()
            .unwrap();

        match &config.kora.usage_limit.rules[0] {
            UsageLimitRuleConfig::Instruction { program, instruction, max, window_seconds } => {
                assert_eq!(program, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                assert_eq!(instruction, "Burn");
                assert_eq!(*max, 5);
                assert_eq!(*window_seconds, None);
            }
            _ => panic!("Expected Instruction rule"),
        }
    }

    #[test]
    fn test_build_transaction_rule() {
        let config = UsageLimitRuleConfig::Transaction { max: 100, window_seconds: Some(3600) };
        let rule = config.build().unwrap();

        assert_eq!(rule.rule_type(), "transaction");
        assert_eq!(rule.max(), 100);
        assert_eq!(rule.window_seconds(), Some(3600));
    }

    #[test]
    fn test_build_instruction_rule() {
        let config = UsageLimitRuleConfig::Instruction {
            program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            instruction: "Transfer".to_string(),
            max: 10,
            window_seconds: None,
        };
        let rule = config.build().unwrap();

        assert_eq!(rule.rule_type(), "instruction");
        assert_eq!(rule.max(), 10);
        assert_eq!(rule.window_seconds(), None);
    }

    #[test]
    fn test_build_instruction_rule_invalid_program() {
        let config = UsageLimitRuleConfig::Instruction {
            program: "invalid".to_string(),
            instruction: "Transfer".to_string(),
            max: 10,
            window_seconds: None,
        };
        assert!(config.build().is_err());
    }

    #[test]
    fn test_usage_limit_config_build_rules() {
        let config = UsageLimitConfig {
            enabled: true,
            cache_url: None,
            fallback_if_unavailable: true,
            rules: vec![
                UsageLimitRuleConfig::Transaction { max: 100, window_seconds: None },
                UsageLimitRuleConfig::Instruction {
                    program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    instruction: "Transfer".to_string(),
                    max: 10,
                    window_seconds: Some(86400),
                },
            ],
        };

        let rules = config.build_rules().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule_type(), "transaction");
        assert_eq!(rules[1].rule_type(), "instruction");
    }
}
