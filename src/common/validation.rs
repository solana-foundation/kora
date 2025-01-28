use crate::common::{config::ValidationConfig, KoraError, instructions::ProgramInstructionConfigWithDiscriminators};
use solana_sdk::{
    instruction::CompiledInstruction, message::Message, pubkey::Pubkey, system_instruction,
    system_program, transaction::Transaction,
};
use std::str::FromStr;

pub enum ValidationMode {
    Sign,
    SignAndSend,
}

pub struct TransactionValidator {
    fee_payer_pubkey: Pubkey,
    max_allowed_lamports: u64,
    allowed_programs: Vec<Pubkey>,
    allowed_program_instructions: Vec<ProgramInstructionConfigWithDiscriminators>,
    max_signatures: usize,
    allowed_tokens: Vec<Pubkey>,
    disallowed_accounts: Vec<Pubkey>,
}

impl TransactionValidator {
    pub fn new(fee_payer_pubkey: Pubkey, config: &ValidationConfig) -> Result<Self, KoraError> {
        // Convert string program IDs to Pubkeys
        let allowed_programs = config
            .allowed_programs
            .iter()
            .map(|addr| {
                Pubkey::from_str(addr).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid program address in config: {}",
                        e
                    ))
                })
            })
            .collect::<Result<Vec<Pubkey>, KoraError>>()?;

        Ok(Self {
            fee_payer_pubkey,
            max_allowed_lamports: config.max_allowed_lamports,
            allowed_programs,
            max_signatures: config.max_signatures,
            allowed_tokens: config
                .allowed_tokens
                .iter()
                .map(|addr| Pubkey::from_str(addr).unwrap())
                .collect(),
            allowed_program_instructions: config.allowed_program_instructions.clone(),
            disallowed_accounts: config
                .disallowed_accounts
                .iter()
                .map(|addr| Pubkey::from_str(addr).unwrap())
                .collect(),
        })
    }

    pub fn validate_token_mint(&self, mint: &Pubkey) -> Result<(), KoraError> {
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {} is not a valid token mint",
                mint
            )));
        }
        Ok(())
    }

    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), KoraError> {
        self.validate_programs(&transaction.message)?;
        self.validate_program_instructions(&transaction.message)?;
        self.validate_transfer_amounts(&transaction.message)?;
        self.validate_signatures(transaction)?;
        self.validate_disallowed_accounts(&transaction.message)?;

        if transaction.message.instructions.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no instructions".to_string(),
            ));
        }

        if transaction.message.account_keys.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no account keys".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate_lamport_fee(&self, fee: u64) -> Result<(), KoraError> {
        if fee > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Fee {} exceeds maximum allowed {}",
                fee, self.max_allowed_lamports
            )));
        }
        Ok(())
    }

    fn validate_signatures(&self, message: &Transaction) -> Result<(), KoraError> {
        if message.signatures.len() > self.max_signatures {
            return Err(KoraError::InvalidTransaction(format!(
                "Too many signatures: {} > {}",
                message.signatures.len(),
                self.max_signatures
            )));
        }

        if message.signatures.is_empty() {
            return Err(KoraError::InvalidTransaction("No signatures found".to_string()));
        }

        Ok(())
    }

    fn validate_programs(&self, message: &Message) -> Result<(), KoraError> {
        for instruction in &message.instructions {
            let program_id = message.account_keys[instruction.program_id_index as usize];
            if !self.allowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is not in the allowed list",
                    program_id
                )));
            }
        }
        Ok(())
    }

    fn validate_program_instructions(&self, message: &Message) -> Result<(), KoraError> {
        for instruction in &message.instructions {
            let program_id = message.account_keys[instruction.program_id_index as usize];
            
            // Find program config in allowed instructions
            let program_config = self.allowed_program_instructions
                .iter()
                .find(|config| Pubkey::from_str(&config.program_id).unwrap() == program_id)
                .ok_or_else(|| KoraError::InvalidTransaction(
                    format!("Program {} not found in allowed instructions", program_id)
                ))?;

            // If wildcard "*" is present, allow all instructions for this program
            if program_config.instructions.contains(&"*".to_string()) {
                continue;
            }

            // Check if instruction data matches any allowed discriminator
            let instruction_data = &instruction.data[..8];  // First 8 bytes are the discriminator
            let instruction_data_vec = instruction_data.to_vec();

            let is_allowed = program_config.discriminators.iter().any(|(disc, _)| {
                disc == &instruction_data_vec
            });

            if !is_allowed {
                return Err(KoraError::InvalidTransaction(
                    format!("Instruction {:?} not allowed for program {}", instruction_data, program_id)
                ));
            }
        }
        Ok(())
    }

    fn validate_fee_payer_usage(&self, message: &Message) -> Result<(), KoraError> {
        // Check if fee payer is first account
        if message.account_keys.first() != Some(&self.fee_payer_pubkey) {
            return Err(KoraError::InvalidTransaction(
                "Fee payer must be the first account".to_string(),
            ));
        }

        // Ensure fee payer is not being used as a source of funds
        for instruction in &message.instructions {
            if self.is_fee_payer_source(instruction, &message.account_keys) {
                return Err(KoraError::InvalidTransaction(
                    "Fee payer cannot be used as source account".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn is_fee_payer_source(&self, ix: &CompiledInstruction, account_keys: &[Pubkey]) -> bool {
        // For system program transfers, check if fee payer is the source
        if account_keys[ix.program_id_index as usize] == system_program::ID {
            if let Ok(system_ix) =
                bincode::deserialize::<system_instruction::SystemInstruction>(&ix.data)
            {
                if let system_instruction::SystemInstruction::Transfer { lamports: _ } = system_ix {
                    // For transfer instruction, first account is source
                    return account_keys[ix.accounts[0] as usize] == self.fee_payer_pubkey;
                }
            }
        }

        false
    }

    fn validate_transfer_amounts(&self, message: &Message) -> Result<(), KoraError> {
        let total_outflow = self.calculate_total_outflow(message);

        if total_outflow > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                total_outflow, self.max_allowed_lamports
            )));
        }

        Ok(())
    }

    pub fn validate_disallowed_accounts(&self, message: &Message) -> Result<(), KoraError> {
        for instruction in &message.instructions {
            // iterate over all accounts in the instruction
            for account in instruction.accounts.iter() {
                let account = message.account_keys[*account as usize];
                if self.disallowed_accounts.contains(&account) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Account {} is disallowed",
                        account
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    fn calculate_total_outflow(&self, message: &Message) -> u64 {
        let mut total = 0u64;

        for instruction in &message.instructions {
            let program_id = message.account_keys[instruction.program_id_index as usize];

            // Handle System Program transfers
            if program_id == system_program::ID {
                if let Ok(system_ix) =
                    bincode::deserialize::<system_instruction::SystemInstruction>(&instruction.data)
                {
                    if let system_instruction::SystemInstruction::Transfer { lamports } = system_ix
                    {
                        // Only count if source is fee payer
                        if message.account_keys[instruction.accounts[0] as usize]
                            == self.fee_payer_pubkey
                        {
                            total = total.saturating_add(lamports);
                        }
                    }
                }
            }
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{message::Message, system_instruction};

    #[test]
    fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        // Test case 1: Transaction using fee payer as source
        let recipient = Pubkey::new_unique();
        let instruction = system_instruction::transfer(&fee_payer, &recipient, 5_000_000);
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_err());

        // Test case 2: Valid transaction within limits
        let sender = Pubkey::new_unique();
        let instruction = system_instruction::transfer(&sender, &recipient, 100_000);
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_transfer_amount_limits() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test transaction with amount over limit
        let instruction = system_instruction::transfer(&sender, &recipient, 2_000_000);
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok()); // Should pass because sender is not fee payer

        // Test multiple transfers
        let instructions = vec![
            system_instruction::transfer(&sender, &recipient, 500_000),
            system_instruction::transfer(&sender, &recipient, 500_000),
        ];
        let message = Message::new(&instructions, Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_validate_programs() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()], // System program
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test allowed program (system program)
        let instruction = system_instruction::transfer(&sender, &recipient, 1000);
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test disallowed program
        let fake_program = Pubkey::new_unique();
        // Create a no-op instruction for the fake program
        let instruction = solana_sdk::instruction::Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_validate_signatures() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 2,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test too many signatures
        let instructions = vec![
            system_instruction::transfer(&sender, &recipient, 1000),
            system_instruction::transfer(&sender, &recipient, 1000),
            system_instruction::transfer(&sender, &recipient, 1000),
        ];
        let message = Message::new(&instructions, Some(&fee_payer));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.signatures = vec![Default::default(); 3]; // Add 3 dummy signatures
        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_sign_and_send_transaction_mode() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test SignAndSend mode with fee payer already set should not error
        let instruction = system_instruction::transfer(&sender, &recipient, 1000);
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test SignAndSend mode without fee payer (should succeed)
        let instruction = system_instruction::transfer(&sender, &recipient, 1000);
        let message = Message::new(&[instruction], None); // No fee payer specified
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_empty_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        // Create an empty message using Message::new with empty instructions
        let message = Message::new(&[], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_disallowed_accounts() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_program_instructions: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec!["hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()],
        };

        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let instruction = system_instruction::transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = Message::new(&[instruction], Some(&fee_payer));
        let transaction = Transaction::new_unsigned(message);
        assert!(validator.validate_transaction(&transaction).is_err());
    }
}
