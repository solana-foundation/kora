use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

use solana_client::nonblocking::rpc_client::RpcClient;

use crate::{error::KoraError, token::check_valid_tokens};
use crate::transaction::instructions::{get_program_instruction_configs_with_discriminators, ProgramInstructionConfig, ProgramInstructionConfigWithDiscriminators};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub validation: RawValidationConfig,
    pub kora: KoraConfig,
}

// Raw validation config loaded directly from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawValidationConfig {
    pub max_allowed_lamports: u64,
    pub max_signatures: usize,
    pub allowed_programs: Vec<String>,
    pub allowed_tokens: Vec<String>,
    #[serde(rename = "allowed_instructions")]
    pub allowed_program_instructions: Vec<ProgramInstructionConfig>,
    pub allowed_spl_paid_tokens: Vec<String>,
    pub disallowed_accounts: Vec<String>,
}

// Runtime validation config with discriminators for instruction validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub max_allowed_lamports: u64,
    pub max_signatures: usize,
    pub allowed_programs: Vec<String>,
    pub allowed_tokens: Vec<String>,
    pub allowed_program_instructions: Vec<ProgramInstructionConfigWithDiscriminators>,
    pub allowed_spl_paid_tokens: Vec<String>,
    pub disallowed_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoraConfig {
    pub rate_limit: u64,
    // pub redis_url: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        KoraError::InternalServerError(format!("Failed to read config file: {}", e))
    })?;

    toml::from_str(&contents)
        .map_err(|e| KoraError::InternalServerError(format!("Failed to parse config file: {}", e)))
}

impl Config {
    pub async fn validate(&self, rpc_client: &RpcClient) -> Result<ValidatedConfig, KoraError> {
        if self.validation.allowed_tokens.is_empty() {
            return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
        }

        let program_instructions_with_discriminators = get_program_instruction_configs_with_discriminators(
            rpc_client,
            &self.validation.allowed_program_instructions
        ).await?;

        check_valid_tokens(rpc_client, &self.validation.allowed_tokens).await?;

        Ok(ValidatedConfig {
            validation: ValidationConfig {
                max_allowed_lamports: self.validation.max_allowed_lamports,
                max_signatures: self.validation.max_signatures,
                allowed_programs: self.validation.allowed_programs.clone(),
                allowed_tokens: self.validation.allowed_tokens.clone(),
                allowed_program_instructions: program_instructions_with_discriminators,
                allowed_spl_paid_tokens: self.validation.allowed_spl_paid_tokens.clone(),
                disallowed_accounts: self.validation.disallowed_accounts.clone(),
            },
            kora: self.kora.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
            [validation]
            max_allowed_lamports = 1000000000
            max_signatures = 10
            allowed_programs = ["program1", "program2"]
            allowed_tokens = ["token1", "token2"]
            allowed_spl_paid_tokens = ["token3"]
            disallowed_accounts = ["account1"]

            [kora]
            rate_limit = 100
        "#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, config_content).unwrap();

        let config = load_config(temp_file.path()).unwrap();

        assert_eq!(config.validation.max_allowed_lamports, 1000000000);
        assert_eq!(config.validation.max_signatures, 10);
        assert_eq!(config.validation.allowed_programs, vec!["program1", "program2"]);
        assert_eq!(config.validation.allowed_tokens, vec!["token1", "token2"]);
        assert_eq!(config.validation.allowed_spl_paid_tokens, vec!["token3"]);
        assert_eq!(config.validation.disallowed_accounts, vec!["account1"]);
        assert_eq!(config.kora.rate_limit, 100);
    }

    #[test]
    fn test_load_invalid_config() {
        let invalid_content = "invalid toml content";
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, invalid_content).unwrap();

        let result = load_config(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_config("nonexistent_file.toml");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config() {
        let mut config = Config {
            validation: RawValidationConfig {
                max_allowed_lamports: 1000000000,
                max_signatures: 10,
                allowed_programs: vec!["program1".to_string()],
                allowed_tokens: vec!["token1".to_string()],
                allowed_program_instructions: vec![],
                allowed_spl_paid_tokens: vec!["token3".to_string()],
                disallowed_accounts: vec!["account1".to_string()],
            },
            kora: KoraConfig { rate_limit: 100 },
        };

        // Test empty tokens list
        config.validation.allowed_tokens.clear();
        let rpc_client = RpcClient::new("http://localhost:8899".to_string());
        let result = config.validate(&rpc_client).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InternalServerError(_)));
    }
}
