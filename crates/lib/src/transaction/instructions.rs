use anyhow::Result;
use borsh::BorshDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use flate2::read::ZlibDecoder;
use std::io::Read;

// Anchor IDL account discriminator
const ANCHOR_DISCRIMINATOR: &str = "anchor:idl";

use serde::{Deserialize, Serialize};

use crate::error::KoraError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInstructionConfig {
    pub program_id: String,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInstructionConfigWithDiscriminators {
    pub program_id: String,
    pub instructions: Vec<String>, // Keep human readable format
    pub discriminators: HashMap<Vec<u8>, String>, // Map instruction bytes to name/raw disc
}

pub async fn get_program_instruction_configs_with_discriminators(
    rpc_client: &RpcClient,
    program_configs: &[ProgramInstructionConfig],
) -> Result<Vec<ProgramInstructionConfigWithDiscriminators>, KoraError> {
    let mut configs_with_discriminators = Vec::new();

    for config in program_configs {
        let mut discriminator_map = HashMap::new();
        let mut instruction_names = Vec::new();

        // Process each instruction - could be raw discriminator or name
        for instruction in &config.instructions {
            if instruction == "*" {
                // Wildcard case - allow all instructions
                instruction_names.push(instruction.clone());
                continue;
            }

            if instruction.starts_with('[') && instruction.ends_with(']') {
                // Raw discriminator case
                let bytes: Vec<u8> = instruction
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                
                discriminator_map.insert(bytes, instruction.clone());
                instruction_names.push(instruction.clone());
            } else {
                // Instruction name case - will look up discriminator from IDL
                instruction_names.push(instruction.clone());
            }
        }

        // Get discriminators for named instructions if any exist
        let named_instructions: Vec<String> = instruction_names
            .iter()
            .filter(|i| !i.starts_with('[') && **i != "*")
            .cloned()
            .collect();

        if !named_instructions.is_empty() {
            let idl_discriminators = get_instruction_discriminators(
                rpc_client,
                &config.program_id,
                &named_instructions,
            ).await?;
            
            // Flip key-value pairs when extending
            for (name, disc) in idl_discriminators {
                discriminator_map.insert(disc, name);
            }
        }

        configs_with_discriminators.push(ProgramInstructionConfigWithDiscriminators {
            program_id: config.program_id.clone(),
            instructions: instruction_names,
            discriminators: discriminator_map,
        });
    }

    Ok(configs_with_discriminators)
}



/// Gets the on-chain IDL for a program and matches instruction discriminators
pub async fn get_instruction_discriminators(
    rpc_client: &RpcClient,
    program_id: &str,
    instruction_names: &[String],
) -> Result<HashMap<String, Vec<u8>>> {
    let program_pubkey = Pubkey::from_str(program_id)?;
    
    // Find account that contains the IDL
    // Taken from https://github.com/coral-xyz/anchor/blob/5d0fe35a455bc98d83e03cecdd257dfb91e8f820/lang/syn/src/codegen/program/idl.rs#L17-L26
    let program_signer = Pubkey::find_program_address(&[], &program_pubkey).0;
    let idl_address = Pubkey::create_with_seed(&program_signer, ANCHOR_DISCRIMINATOR, &program_pubkey)?;

    // Get IDL account data
    let idl_account = rpc_client.get_account(&idl_address).await?;
    
    // Parse IDL data
    // TODO: v0.31 and above versions of anchor do not require the discriminator to be 8 bytes...
    let mut d: &[u8] = &idl_account.data[8..]; // Skip 8-byte discriminator
    
    #[derive(BorshDeserialize)]
    struct IdlAccountData {
        authority: Pubkey,
        data_len: u32,
        data: Vec<u8>,
    }
    
    let idl_account: IdlAccountData = borsh::BorshDeserialize::deserialize(&mut d)?;

    let compressed_len: usize = idl_account.data_len.try_into().unwrap();
    let compressed_bytes = &idl_account.data[..compressed_len];
    let mut z = ZlibDecoder::new(compressed_bytes);
    let mut s = Vec::new();
    z.read_to_end(&mut s)?;
    let idl: serde_json::Value = serde_json::from_slice(&s[..])?;

    let mut discriminators = HashMap::new();

    // Get instructions from IDL
    if let Some(instructions) = idl["instructions"].as_array() {
        for instruction in instructions {
            if let Some(name) = instruction["name"].as_str() {
                // Convert instruction name to snake_case for matching
                let snake_case = to_snake_case(name);
                
                // Check if this instruction name is in our allowed list
                if instruction_names.contains(&snake_case) {
                    // Calculate anchor discriminator
                    let mut discriminator = [0u8; 8];
                    let preimage = format!("global:{}", name);
                    let hash = solana_program::hash::hash(preimage.as_bytes());
                    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
                    
                    discriminators.insert(snake_case, discriminator.to_vec());
                }
            }
        }
    }

    Ok(discriminators)
}

// Helper to convert camelCase to snake_case
fn to_snake_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !result.is_empty() && chars.peek().map_or(false, |next| next.is_lowercase()) {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("transferChecked"), "transfer_checked");
        assert_eq!(to_snake_case("transfer"), "transfer");
        assert_eq!(to_snake_case("initializeAccount"), "initialize_account");
    }
}