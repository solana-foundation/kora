use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

use crate::common::{token::check_valid_tokens, KoraError};
use crate::common::instructions::{get_program_instruction_configs_with_discriminators, ProgramInstructionConfig, ProgramInstructionConfigWithDiscriminators};
use solana_client::nonblocking::rpc_client::RpcClient;

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
#[derive(Debug, Clone, Serialize)]
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
            log::error!("No tokens enabled");
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

#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub validation: ValidationConfig,
    pub kora: KoraConfig,
}
