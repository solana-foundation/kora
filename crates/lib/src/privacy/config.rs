//! Privacy-specific configuration.
//!
//! This module defines configuration for CPI-based fee payments from
//! privacy pool programs.

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use utoipa::ToSchema;

/// Configuration for privacy pool CPI-based fee payments.
///
/// When enabled, Kora will accept fee payments via CPI transfers from
/// programs listed in `allowed_fee_payment_programs`, in addition to
/// standard top-level token transfers.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivacyConfig {
    /// Enable CPI-based fee payment validation.
    ///
    /// When enabled, fee payments can come from CPI transfers
    /// initiated by programs in `allowed_fee_payment_programs`.
    #[serde(default)]
    pub enabled: bool,

    /// Programs allowed to emit CPI fee payments.
    ///
    /// Typically just the privacy pool program ID. Any SPL token transfer
    /// CPI originating from these programs will be considered valid fee payment.
    #[serde(default)]
    pub allowed_fee_payment_programs: Vec<String>,

    /// Cached parsed pubkeys (populated on initialization).
    #[serde(skip)]
    parsed_programs: Option<Vec<Pubkey>>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_fee_payment_programs: Vec::new(),
            parsed_programs: None,
        }
    }
}

impl PrivacyConfig {
    /// Initialize and parse program addresses.
    ///
    /// This should be called after deserialization to populate the cached
    /// pubkey fields. Returns an error if any address is invalid.
    pub fn initialize(&mut self) -> Result<(), String> {
        let mut programs = Vec::with_capacity(self.allowed_fee_payment_programs.len());

        for addr in &self.allowed_fee_payment_programs {
            match Pubkey::from_str(addr) {
                Ok(pubkey) => programs.push(pubkey),
                Err(e) => {
                    return Err(format!("Invalid program address '{}': {}", addr, e));
                }
            }
        }

        self.parsed_programs = Some(programs);
        Ok(())
    }

    /// Get allowed fee payment programs as Pubkeys.
    ///
    /// Returns an empty slice if not initialized or no programs configured.
    pub fn get_allowed_programs(&self) -> &[Pubkey] {
        self.parsed_programs.as_deref().unwrap_or(&[])
    }

    /// Check if a program is allowed to emit fee payments via CPI.
    pub fn is_program_allowed(&self, program_id: &Pubkey) -> bool {
        self.get_allowed_programs().contains(program_id)
    }

    /// Check if privacy mode is enabled and properly configured.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.get_allowed_programs().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config(enabled: bool, programs: Vec<&str>) -> PrivacyConfig {
        let mut config = PrivacyConfig {
            enabled,
            allowed_fee_payment_programs: programs.into_iter().map(String::from).collect(),
            parsed_programs: None,
        };
        let _ = config.initialize();
        config
    }

    #[test]
    fn test_default_config() {
        let config = PrivacyConfig::default();
        assert!(!config.enabled);
        assert!(config.allowed_fee_payment_programs.is_empty());
        assert!(!config.is_active());
    }

    #[test]
    fn test_initialization_valid_address() {
        let mut config = PrivacyConfig {
            enabled: true,
            allowed_fee_payment_programs: vec![
                "11111111111111111111111111111111".to_string(),
            ],
            parsed_programs: None,
        };

        assert!(config.initialize().is_ok());
        assert_eq!(config.get_allowed_programs().len(), 1);
        assert!(config.is_active());
    }

    #[test]
    fn test_initialization_invalid_address() {
        let mut config = PrivacyConfig {
            enabled: true,
            allowed_fee_payment_programs: vec!["invalid_address".to_string()],
            parsed_programs: None,
        };

        let result = config.initialize();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid program address"));
    }

    #[test]
    fn test_initialization_multiple_addresses() {
        let program1 = Pubkey::new_unique();
        let program2 = Pubkey::new_unique();

        let mut config = PrivacyConfig {
            enabled: true,
            allowed_fee_payment_programs: vec![program1.to_string(), program2.to_string()],
            parsed_programs: None,
        };

        assert!(config.initialize().is_ok());
        assert_eq!(config.get_allowed_programs().len(), 2);
        assert!(config.is_program_allowed(&program1));
        assert!(config.is_program_allowed(&program2));
    }

    #[test]
    fn test_is_program_allowed() {
        let allowed_program = Pubkey::new_unique();
        let random_program = Pubkey::new_unique();

        let config = make_test_config(true, vec![&allowed_program.to_string()]);

        assert!(config.is_program_allowed(&allowed_program));
        assert!(!config.is_program_allowed(&random_program));
    }

    #[test]
    fn test_is_active_requires_enabled_and_programs() {
        // Enabled but no programs
        let config1 = make_test_config(true, vec![]);
        assert!(!config1.is_active());

        // Programs but not enabled
        let mut config2 = PrivacyConfig {
            enabled: false,
            allowed_fee_payment_programs: vec!["11111111111111111111111111111111".to_string()],
            parsed_programs: None,
        };
        config2.initialize().unwrap();
        assert!(!config2.is_active());

        // Both enabled and has programs
        let config3 = make_test_config(true, vec!["11111111111111111111111111111111"]);
        assert!(config3.is_active());
    }

    #[test]
    fn test_get_allowed_programs_not_initialized() {
        let config = PrivacyConfig {
            enabled: true,
            allowed_fee_payment_programs: vec!["11111111111111111111111111111111".to_string()],
            parsed_programs: None,
        };

        // Should return empty slice when not initialized
        assert!(config.get_allowed_programs().is_empty());
    }
}
