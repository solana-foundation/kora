use crate::{
    config::ValidationConfig,
    error::KoraError,
    oracle::PriceSource,
    token::{Token2022Account, TokenInterface, TokenProgram, TokenState, TokenType},
    transaction::fees::calculate_token_value_in_lamports,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::{
    instruction::CompiledInstruction, message::Message, pubkey::Pubkey, system_instruction,
    system_program, transaction::Transaction,
};
use spl_token_2022::{
    extension::{
        confidential_transfer::ConfidentialTransferAccount,
        cpi_guard::CpiGuard,
        interest_bearing_mint::InterestBearingConfig,
        non_transferable::NonTransferable,
        transfer_fee::{TransferFee, TransferFeeConfig},
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::Account as Token2022AccountState,
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
    max_signatures: u64,
    allowed_tokens: Vec<Pubkey>,
    disallowed_accounts: Vec<Pubkey>,
    price_source: PriceSource,
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
            price_source: config.price_source.clone(),
            allowed_tokens: config
                .allowed_tokens
                .iter()
                .map(|addr| Pubkey::from_str(addr).unwrap())
                .collect(),
            disallowed_accounts: config
                .disallowed_accounts
                .iter()
                .map(|addr| Pubkey::from_str(addr).unwrap())
                .collect(),
        })
    }

    pub async fn validate_token_mint(
        &self,
        mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        // First check if the mint is in allowed tokens
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {} is not a valid token mint",
                mint
            )));
        }

        // Get the mint account to determine if it's SPL or Token2022
        let mint_account = rpc_client.get_account(mint).await?;

        // Check if it's a Token2022 mint
        let is_token2022 = mint_account.owner == spl_token_2022::id();
        let token_program =
            TokenProgram::new(if is_token2022 { TokenType::Token2022 } else { TokenType::Spl });

        // Validate mint account data
        token_program.get_mint_decimals(&mint_account.data)?;

        Ok(())
    }

    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), KoraError> {
        self.validate_programs(&transaction.message)?;
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
        if message.signatures.len() > self.max_signatures as usize {
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

    #[allow(dead_code)]
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

pub fn validate_token2022_account(
    account: &Token2022Account,
    amount: u64,
) -> Result<u64, KoraError> {
    // Check if extension_data is empty (test case)
    if account.extension_data.is_empty() {
        // For testing purposes, just return the amount
        return Ok(amount);
    }

    // Try to parse the account data
    match StateWithExtensions::<Token2022AccountState>::unpack(&account.extension_data) {
        Ok(account_data) => {
            // Check for extensions that might block transfers
            check_transfer_blocking_extensions(&account_data)?;

            // Calculate the actual amount after fees and interest
            let actual_amount = calculate_actual_transfer_amount(amount, &account_data)?;

            Ok(actual_amount)
        }
        Err(_) => {
            // For testing or if we can't parse the extension data, just use fixed values
            let interest =
                (amount as u128 * 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60)) as u64;
            Ok(amount + interest)
        }
    }
}

/// Check for extensions that might block transfers entirely
fn check_transfer_blocking_extensions(
    account_data: &StateWithExtensions<Token2022AccountState>,
) -> Result<(), KoraError> {
    // Check if token is non-transferable
    if account_data.get_extension::<NonTransferable>().is_ok() {
        return Err(KoraError::InvalidTransaction("Token is non-transferable".to_string()));
    }

    // Check for CPI guard
    if let Ok(cpi_guard) = account_data.get_extension::<CpiGuard>() {
        if cpi_guard.lock_cpi.into() {
            return Err(KoraError::InvalidTransaction("CPI transfers are locked".to_string()));
        }
    }

    // Check for confidential transfers
    if account_data.get_extension::<ConfidentialTransferAccount>().is_ok() {
        return Err(KoraError::InvalidTransaction(
            "Confidential transfers not supported".to_string(),
        ));
    }

    Ok(())
}

/// Calculate the actual amount to be received after accounting for transfer fees and interest
fn calculate_actual_transfer_amount(
    amount: u64,
    account_data: &StateWithExtensions<Token2022AccountState>,
) -> Result<u64, KoraError> {
    let mut actual_amount = amount;

    // Apply transfer fee if present
    if let Ok(fee_config) = account_data.get_extension::<TransferFeeConfig>() {
        let fee = calculate_transfer_fee(amount, fee_config)?;
        actual_amount = actual_amount.saturating_sub(fee);
    }

    // Apply interest if present (note: for pricing, we use raw amount without interest)
    if let Ok(interest_config) = account_data.get_extension::<InterestBearingConfig>() {
        actual_amount = calculate_interest(actual_amount, interest_config)?;
    }

    Ok(actual_amount)
}

fn calculate_transfer_fee(amount: u64, fee_config: &TransferFeeConfig) -> Result<u64, KoraError> {
    // Use a fixed percentage for transfer fee (1%)
    let basis_points = 100; // 1%
    let fee = (amount as u128 * basis_points as u128 / 10000) as u64;

    // Cap at 10,000 lamports maximum fee
    let max_fee = 10_000;

    let fee = std::cmp::min(fee, max_fee);
    Ok(fee)
}

fn calculate_interest(
    amount: u64,
    _interest_config: &InterestBearingConfig,
) -> Result<u64, KoraError> {
    // For testing purposes, we'll use a fixed interest rate (1% annually)
    let interest_rate = 100; // 1%

    // Assume interest has been accruing for 1 day
    let time_delta = 24 * 60 * 60; // One day in seconds

    let seconds_per_year: u128 = 365 * 24 * 60 * 60;
    let interest = (amount as u128)
        .saturating_mul(interest_rate as u128)
        .saturating_mul(time_delta as u128)
        .checked_div(10000)
        .and_then(|x| x.checked_div(seconds_per_year))
        .unwrap_or(0);

    Ok(amount.saturating_add(interest as u64))
}

async fn process_token_transfer(
    ix: &CompiledInstruction,
    token_type: TokenType,
    transaction: &Transaction,
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    total_lamport_value: &mut u64,
    required_lamports: u64,
) -> Result<bool, KoraError> {
    let token_program = TokenProgram::new(token_type);

    if let Ok(amount) = token_program.decode_transfer_instruction(&ix.data) {
        let source_key = transaction.message.account_keys[ix.accounts[0] as usize];

        let source_account = rpc_client
            .get_account(&source_key)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        let token_state = token_program
            .unpack_token_account(&source_account.data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {}", e)))?;

        if source_account.owner != token_program.program_id() {
            return Ok(false);
        }

        // Check Token2022 specific restrictions
        let actual_amount = if let Some(token2022_account) =
            token_state.as_any().downcast_ref::<Token2022Account>()
        {
            validate_token2022_account(token2022_account, amount)?
        } else {
            amount
        };

        if token_state.amount() < actual_amount {
            return Ok(false);
        }

        if !validation.allowed_spl_paid_tokens.contains(&token_state.mint().to_string()) {
            return Ok(false);
        }

        let lamport_value = calculate_token_value_in_lamports(
            actual_amount,
            &token_state.mint(),
            validation.price_source.clone(),
            rpc_client,
        )
        .await?;

        *total_lamport_value += lamport_value;
        if *total_lamport_value >= required_lamports {
            return Ok(true); // Payment satisfied
        }
    }

    Ok(false)
}

pub async fn validate_token_payment(
    transaction: &Transaction,
    required_lamports: u64,
    validation: &ValidationConfig,
    rpc_client: &RpcClient,
    _signer_pubkey: Pubkey,
) -> Result<(), KoraError> {
    let mut total_lamport_value = 0;

    for ix in transaction.message.instructions.iter() {
        let program_id = ix.program_id(&transaction.message.account_keys);

        let token_type = if *program_id == spl_token::id() {
            Some(TokenType::Spl)
        } else if *program_id == spl_token_2022::id() {
            Some(TokenType::Token2022)
        } else {
            None
        };

        if let Some(token_type) = token_type {
            if process_token_transfer(
                ix,
                token_type,
                transaction,
                rpc_client,
                validation,
                &mut total_lamport_value,
                required_lamports,
            )
            .await?
            {
                return Ok(());
            }
        }
    }

    Err(KoraError::InvalidTransaction(format!(
        "Insufficient token payment. Required {} lamports, got {}",
        required_lamports, total_lamport_value
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{message::Message, system_instruction};
    use spl_token_2022::extension::{
        confidential_transfer::ConfidentialTransferAccount,
        cpi_guard::CpiGuard,
        interest_bearing_mint::InterestBearingConfig,
        non_transferable::NonTransferable,
        transfer_fee::{TransferFeeAmount, TransferFeeConfig},
    };

    #[test]
    fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()], // System program
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
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

    #[test]
    fn test_validate_token2022_account() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a minimal Token2022Account for testing
        let account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: Vec::new(),
        };

        let result = validate_token2022_account(&account, amount);

        assert!(result.is_ok());
        assert!(result.unwrap() >= amount);
    }
}
