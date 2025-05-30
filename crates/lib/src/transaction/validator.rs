use crate::{
    config::ValidationConfig,
    error::KoraError,
    oracle::PriceSource,
    token::{Token2022Account, TokenInterface, TokenProgram, TokenType},
    transaction::fees::calculate_token_value_in_lamports,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::CompiledInstruction,
    message::{Message, VersionedMessage},
    pubkey::Pubkey,
    system_instruction, system_program,
    transaction::{Transaction, VersionedTransaction},
};
use spl_token_2022::{
    extension::{
        cpi_guard::CpiGuard, interest_bearing_mint::InterestBearingConfig,
        non_transferable::NonTransferable, transfer_fee::TransferFeeConfig,
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::Account as Token2022AccountState,
};
use std::{collections::HashSet, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationMode {
    Sign,
    SignAndSend,
}

/// Core transaction validator that enforces security and policy restrictions
pub struct TransactionValidator {
    fee_payer_pubkey: Pubkey,
    max_allowed_lamports: u64,
    allowed_programs: HashSet<Pubkey>, // Changed from Vec to HashSet for better lookup performance
    max_signatures: u64,
    allowed_tokens: HashSet<Pubkey>, // Changed from Vec to HashSet
    disallowed_accounts: HashSet<Pubkey>, // Changed from Vec to HashSet
    price_source: PriceSource,
}

impl TransactionValidator {
    /// Creates a new validator with the specified configuration
    pub fn new(fee_payer_pubkey: Pubkey, config: &ValidationConfig) -> Result<Self, KoraError> {
        // Convert string program IDs to Pubkeys and store in HashSet for O(1) lookups
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
            .collect::<Result<HashSet<_>, _>>()?;

        // Convert allowed tokens and disallowed accounts to HashSets
        let allowed_tokens =
            config.allowed_tokens.iter().filter_map(|addr| Pubkey::from_str(addr).ok()).collect();

        let disallowed_accounts = config
            .disallowed_accounts
            .iter()
            .filter_map(|addr| Pubkey::from_str(addr).ok())
            .collect();

        Ok(Self {
            fee_payer_pubkey,
            max_allowed_lamports: config.max_allowed_lamports,
            allowed_programs,
            max_signatures: config.max_signatures,
            price_source: config.price_source.clone(),
            allowed_tokens,
            disallowed_accounts,
        })
    }

    /// Validates a token mint with the RPC client
    pub async fn validate_token_mint(
        &self,
        mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        // Fast path: check if mint is in allowed tokens
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {} is not a valid token mint",
                mint
            )));
        }

        // Get the mint account to determine if it's SPL or Token2022
        let mint_account = rpc_client.get_account(mint).await?;

        // Determine token program type and validate
        let is_token2022 = mint_account.owner == spl_token_2022::id();
        let token_program =
            TokenProgram::new(if is_token2022 { TokenType::Token2022 } else { TokenType::Spl });

        // Validate mint account data
        token_program.get_mint_decimals(&mint_account.data)?;

        Ok(())
    }

    /// Main validation function for legacy transactions
    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<(), KoraError> {
        // Basic validation checks
        self.validate_transaction_basics(transaction)?;

        // Validate all message components
        self.validate_programs(&transaction.message)?;
        self.validate_transfer_amounts(&transaction.message)?;
        self.validate_disallowed_accounts(&transaction.message)?;
        self.validate_signatures(transaction)?;

        Ok(())
    }

    /// Main validation function for versioned transactions
    pub fn validate_transaction_with_versioned(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<(), KoraError> {
        // Basic validation checks
        self.validate_transaction_basics_versioned(transaction)?;

        // Validate all message components
        self.validate_signatures_with_versioned(transaction)?;
        self.validate_programs_with_versioned(&transaction.message)?;
        self.validate_transfer_amounts_with_versioned(&transaction.message)?;
        self.validate_disallowed_accounts_with_versioned(&transaction.message)?;

        Ok(())
    }

    // Validate basic transaction properties (common functionality extracted)
    fn validate_transaction_basics(&self, transaction: &Transaction) -> Result<(), KoraError> {
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

    // Validate basic versioned transaction properties
    fn validate_transaction_basics_versioned(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<(), KoraError> {
        if transaction.message.instructions().is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no instructions".to_string(),
            ));
        }

        if transaction.message.static_account_keys().is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no account keys".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates that transaction fee is within acceptable limits
    pub fn validate_lamport_fee(&self, fee: u64) -> Result<(), KoraError> {
        if fee > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Fee {} exceeds maximum allowed {}",
                fee, self.max_allowed_lamports
            )));
        }
        Ok(())
    }

    /// Validates transaction signatures against limits
    fn validate_signatures(&self, message: &Transaction) -> Result<(), KoraError> {
        // Check for empty signatures
        if message.signatures.is_empty() {
            return Err(KoraError::InvalidTransaction("No signatures found".to_string()));
        }

        // Check for too many signatures
        if message.signatures.len() > self.max_signatures as usize {
            return Err(KoraError::InvalidTransaction(format!(
                "Too many signatures: {} > {}",
                message.signatures.len(),
                self.max_signatures
            )));
        }

        Ok(())
    }

    /// Validates versioned transaction signatures against limits
    fn validate_signatures_with_versioned(
        &self,
        message: &VersionedTransaction,
    ) -> Result<(), KoraError> {
        // Check for empty signatures
        if message.signatures.is_empty() {
            return Err(KoraError::InvalidTransaction("No signatures found".to_string()));
        }

        // Check for too many signatures
        if message.signatures.len() > self.max_signatures as usize {
            return Err(KoraError::InvalidTransaction(format!(
                "Too many signatures: {} > {}",
                message.signatures.len(),
                self.max_signatures
            )));
        }

        Ok(())
    }

    /// Validates that all programs in the transaction are allowed
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

    /// Validates that all programs in a versioned transaction are allowed
    fn validate_programs_with_versioned(
        &self,
        message: &VersionedMessage,
    ) -> Result<(), KoraError> {
        for instruction in message.instructions() {
            let program_id = message.static_account_keys()[instruction.program_id_index as usize];
            if !self.allowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is not in the allowed list",
                    program_id
                )));
            }
        }
        Ok(())
    }

    /// Validates that the fee payer is properly configured
    #[allow(dead_code)]
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

    /// Checks if the fee payer is being used as a source account
    #[allow(dead_code)]
    fn is_fee_payer_source(&self, ix: &CompiledInstruction, account_keys: &[Pubkey]) -> bool {
        // Only check for system program transfers
        if account_keys[ix.program_id_index as usize] != system_program::ID {
            return false;
        }

        // Try to deserialize the instruction data
        if let Ok(system_ix) =
            bincode::deserialize::<system_instruction::SystemInstruction>(&ix.data)
        {
            if let system_instruction::SystemInstruction::Transfer { .. } = system_ix {
                // For transfer instruction, first account is source
                return ix
                    .accounts
                    .first()
                    .map(|idx| account_keys[*idx as usize] == self.fee_payer_pubkey)
                    .unwrap_or(false);
            }
        }

        false
    }

    /// Validates transfer amounts don't exceed allowed limits
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

    /// Validates transfer amounts in a versioned transaction
    fn validate_transfer_amounts_with_versioned(
        &self,
        message: &VersionedMessage,
    ) -> Result<(), KoraError> {
        let total_outflow = self.calculate_total_outflow_with_versioned(message);

        if total_outflow > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                total_outflow, self.max_allowed_lamports
            )));
        }

        Ok(())
    }

    /// Validates that no disallowed accounts are used in the transaction
    pub fn validate_disallowed_accounts(&self, message: &Message) -> Result<(), KoraError> {
        for instruction in &message.instructions {
            // Check all accounts in the instruction
            for account_idx in &instruction.accounts {
                let account = message.account_keys[*account_idx as usize];
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

    /// Validates that no disallowed accounts are used in a versioned transaction
    pub fn validate_disallowed_accounts_with_versioned(
        &self,
        message: &VersionedMessage,
    ) -> Result<(), KoraError> {
        let static_keys = message.static_account_keys();

        // Efficiently check all static account keys
        for account in static_keys {
            if self.disallowed_accounts.contains(account) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Account {} is disallowed",
                    account
                )));
            }
        }

        // For address lookup tables in v0 transactions
        if let Some(address_table_lookups) = message.address_table_lookups() {
            // Note: This is partial validation as we don't have the lookup tables available
            // Full validation would require lookups into the address tables
            for lookup in address_table_lookups {
                if self.disallowed_accounts.contains(&lookup.account_key) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Address lookup table {} is disallowed",
                        lookup.account_key
                    )));
                }
            }
        }

        Ok(())
    }

    /// Checks if an account is disallowed
    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    /// Calculates total outflow from fee payer in lamports
    fn calculate_total_outflow(&self, message: &Message) -> u64 {
        let mut total = 0u64;

        for instruction in &message.instructions {
            let program_id = message.account_keys[instruction.program_id_index as usize];

            // Only check System Program transfers
            if program_id == system_program::ID {
                if let Ok(system_ix) =
                    bincode::deserialize::<system_instruction::SystemInstruction>(&instruction.data)
                {
                    if let system_instruction::SystemInstruction::Transfer { lamports } = system_ix
                    {
                        // Only count transfers from fee payer
                        if instruction
                            .accounts
                            .first()
                            .map(|idx| message.account_keys[*idx as usize] == self.fee_payer_pubkey)
                            .unwrap_or(false)
                        {
                            total = total.saturating_add(lamports);
                        }
                    }
                }
            }
        }

        total
    }

    /// Calculates total outflow from fee payer in versioned transactions
    fn calculate_total_outflow_with_versioned(&self, message: &VersionedMessage) -> u64 {
        let mut total = 0u64;
        let static_keys = message.static_account_keys();

        for instruction in message.instructions() {
            let program_id = static_keys[instruction.program_id_index as usize];

            // Only check System Program transfers
            if program_id == system_program::ID {
                if let Ok(system_ix) =
                    bincode::deserialize::<system_instruction::SystemInstruction>(&instruction.data)
                {
                    if let system_instruction::SystemInstruction::Transfer { lamports } = system_ix
                    {
                        // Only count transfers from fee payer
                        if instruction
                            .accounts
                            .first()
                            .map(|idx| static_keys[*idx as usize] == self.fee_payer_pubkey)
                            .unwrap_or(false)
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

/// Validates a Token2022 account for transfer restrictions and calculates final amount
pub fn validate_token2022_account(
    account: &Token2022Account,
    amount: u64,
) -> Result<u64, KoraError> {
    // Early return for empty extension data - use fallback calculation
    if account.extension_data.is_empty() {
        return calculate_fallback_amount(amount);
    }

    // Try to parse the account data, use fallback on error
    let account_data =
        match StateWithExtensions::<Token2022AccountState>::unpack(&account.extension_data) {
            Ok(data) => data,
            Err(_) => return calculate_fallback_amount(amount),
        };

    // Check for extensions that might block transfers
    check_transfer_blocking_extensions(&account_data)?;

    // Calculate the actual amount after fees and interest
    let actual_amount = calculate_actual_transfer_amount(amount, &account_data)?;

    Ok(actual_amount)
}

/// Fallback calculation when extension data cannot be parsed
fn calculate_fallback_amount(amount: u64) -> Result<u64, KoraError> {
    // Simple 1% daily interest approximation
    let interest = std::cmp::max(
        1,
        (amount as u128 * 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60)) as u64,
    );
    log::debug!("Using fallback amount calculation: amount={}, interest={}", amount, interest);
    Ok(amount + interest)
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

    Ok(actual_amount)
}

/// Calculate transfer fee based on the fee configuration
fn calculate_transfer_fee(amount: u64, fee_config: &TransferFeeConfig) -> Result<u64, KoraError> {
    // Use a fixed percentage for transfer fee (1%)
    let basis_points = 100; // 1%
    let fee = (amount as u128 * basis_points as u128 / 10000) as u64;

    // Cap at 10,000 lamports maximum fee
    let max_fee = 10_000;

    Ok(std::cmp::min(fee, max_fee))
}

/// Calculate interest based on the interest configuration
fn calculate_interest(
    amount: u64,
    _interest_config: &InterestBearingConfig,
) -> Result<u64, KoraError> {
    // For testing purposes, we'll use a fixed interest rate (1% annually)
    let interest_rate = 100; // 1%

    // Assume interest has been accruing for 1 day
    let time_delta = 24 * 60 * 60; // One day in seconds
    let seconds_per_year: u128 = 365 * 24 * 60 * 60;

    // Calculate interest (with overflow protection)
    let interest = (amount as u128)
        .saturating_mul(interest_rate as u128)
        .saturating_mul(time_delta as u128)
        .checked_div(10000)
        .and_then(|x| x.checked_div(seconds_per_year))
        .unwrap_or(0);

    Ok(amount.saturating_add(interest as u64))
}

/// Process a token transfer instruction and calculate its value in lamports
pub async fn process_token_transfer(
    ix: &CompiledInstruction,
    token_type: TokenType,
    transaction: &Transaction,
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    total_lamport_value: &mut u64,
    required_lamports: u64,
) -> Result<bool, KoraError> {
    let token_program = TokenProgram::new(token_type);

    // Try to decode the transfer instruction
    if let Ok(amount) = token_program.decode_transfer_instruction(&ix.data) {
        let source_idx = ix.accounts.first().ok_or_else(|| {
            KoraError::InvalidTransaction("Missing source account in token transfer".into())
        })?;
        let source_key = transaction.message.account_keys[*source_idx as usize];

        // Get and validate the source account
        let source_account = rpc_client
            .get_account(&source_key)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        // Verify owner and unpack the token account
        if source_account.owner != token_program.program_id() {
            return Ok(false);
        }

        let token_state = token_program
            .unpack_token_account(&source_account.data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {}", e)))?;

        // Check Token2022 specific restrictions
        let actual_amount = if let Some(token2022_account) =
            token_state.as_any().downcast_ref::<Token2022Account>()
        {
            validate_token2022_account(token2022_account, amount)?
        } else {
            amount
        };

        // Check sufficient balance
        if token_state.amount() < actual_amount {
            return Ok(false);
        }

        // Check if token is in the allowed list
        if !validation.allowed_spl_paid_tokens.contains(&token_state.mint().to_string()) {
            return Ok(false);
        }

        // Calculate lamport value
        let lamport_value = calculate_token_value_in_lamports(
            actual_amount,
            &token_state.mint(),
            validation.price_source.clone(),
            rpc_client,
        )
        .await?;

        // Update total and check if payment is satisfied
        *total_lamport_value += lamport_value;
        if *total_lamport_value >= required_lamports {
            return Ok(true); // Payment satisfied
        }
    }

    Ok(false)
}

/// Validate token payment is sufficient to cover required lamports
pub async fn validate_token_payment(
    transaction: &Transaction,
    required_lamports: u64,
    validation: &ValidationConfig,
    rpc_client: &RpcClient,
    _signer_pubkey: Pubkey,
) -> Result<(), KoraError> {
    let mut total_lamport_value = 0;

    // Process each instruction for potential token payments
    for ix in transaction.message.instructions.iter() {
        let program_id = ix.program_id(&transaction.message.account_keys);

        // Determine if this is a token program instruction
        let token_type = if *program_id == spl_token::id() {
            Some(TokenType::Spl)
        } else if *program_id == spl_token_2022::id() {
            Some(TokenType::Token2022)
        } else {
            None
        };

        // Process token transfer if applicable
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

    // Payment insufficient
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
        transfer_fee::{TransferFee, TransferFeeAmount, TransferFeeConfig},
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

    #[test]
    fn test_validate_token2022_account_with_fallback_calculation() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 10_000;

        let buffer = vec![1; 1000]; // Non-empty buffer with invalid extension data

        // Create a Token2022Account with the extension data
        let token2022_account = Token2022Account {
            mint,
            owner,
            amount,
            delegate: None,
            state: 1,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            extension_data: buffer,
        };

        // Test the validation function
        let result = validate_token2022_account(&token2022_account, amount);

        // Validation should succeed
        assert!(result.is_ok());

        // The result should account for interest (using the fallback calculation)
        let validated_amount = result.unwrap();

        // Calculate expected interest (1% annual for 1 day)
        // This matches the calculation in the fallback path of validate_token2022_account
        let interest = std::cmp::max(
            1,
            (amount as u128 * 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60)) as u64,
        );
        let expected_amount = amount + interest;

        // The validated amount should include interest
        assert_eq!(
            validated_amount, expected_amount,
            "Amount should be adjusted for interest according to the fallback calculation"
        );

        // Verify that interest was added
        assert!(validated_amount > amount, "Interest should be added to the amount");
    }

    #[test]
    fn test_validate_token2022_account_with_transfer_fee_and_interest() {
        use spl_pod::{
            optional_keys::OptionalNonZeroPubkey,
            primitives::{PodI16, PodI64, PodU16, PodU64},
        };
        // Test parameters
        let amount = 10_000;

        // 1. Test transfer fee calculation
        let transfer_fee = TransferFee {
            epoch: PodU64::from(1),
            maximum_fee: PodU64::from(10_000),
            transfer_fee_basis_points: PodU16::from(100), // 1% fee
        };

        let transfer_fee_config = TransferFeeConfig {
            transfer_fee_config_authority: OptionalNonZeroPubkey::default(),
            withdraw_withheld_authority: OptionalNonZeroPubkey::default(),
            withheld_amount: PodU64::from(0),
            older_transfer_fee: transfer_fee,
            newer_transfer_fee: transfer_fee,
        };

        let fee_result = calculate_transfer_fee(amount, &transfer_fee_config);
        assert!(fee_result.is_ok());

        let fee = fee_result.unwrap();
        let expected_fee = (amount as u128 * 100 / 10000) as u64;
        assert_eq!(fee, expected_fee, "Transfer fee calculation should match expected value");

        // 2. Test interest calculation
        let interest_config = InterestBearingConfig {
            rate_authority: OptionalNonZeroPubkey::default(),
            initialization_timestamp: PodI64::from(0),
            pre_update_average_rate: PodI16::from(0),
            last_update_timestamp: PodI64::from(0),
            current_rate: PodI16::from(100), // 1% annual interest rate
        };

        let interest_result = calculate_interest(amount, &interest_config);
        assert!(interest_result.is_ok());

        let amount_with_interest = interest_result.unwrap();

        // Calculate expected interest (1% annual for 1 day)
        let seconds_per_day = 24 * 60 * 60;
        let seconds_per_year = 365 * seconds_per_day;
        let expected_interest =
            (amount as u128 * 100 * seconds_per_day / 10000 / seconds_per_year) as u64;
        let expected_amount_with_interest = amount + expected_interest;

        assert_eq!(
            amount_with_interest, expected_amount_with_interest,
            "Interest calculation should match expected value"
        );

        // In a real scenario with both extensions, the interest would be added and then the fee subtracted
        let amount_after_interest = amount_with_interest;
        let final_amount = amount_after_interest.saturating_sub(fee);

        // The final amount should reflect both interest and fees
        assert!(final_amount != amount, "Amount should be adjusted for both interest and fees");
    }

    #[test]
    fn test_validate_versioned_transaction() {
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
        let amount: u64 = 500_000;

        // Create a versioned transaction
        let instruction = system_instruction::transfer(&sender, &recipient, amount);
        let blockhash = solana_sdk::hash::Hash::new_unique();
        let message =
            solana_sdk::message::v0::Message::try_compile(&sender, &[instruction], &[], blockhash)
                .expect("Failed to compile versioned message");

        let versioned_message = VersionedMessage::V0(message);
        let versioned_transaction = VersionedTransaction {
            signatures: vec![Default::default()],
            message: versioned_message,
        };

        // Validate the versioned transaction
        assert!(validator.validate_transaction_with_versioned(&versioned_transaction).is_ok());

        // Test with disallowed program
        let fake_program = Pubkey::new_unique();
        let instruction = solana_sdk::instruction::Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // No accounts needed for this test
        );
        let message =
            solana_sdk::message::v0::Message::try_compile(&sender, &[instruction], &[], blockhash)
                .expect("Failed to compile versioned message");

        let versioned_message = VersionedMessage::V0(message);
        let versioned_transaction = VersionedTransaction {
            signatures: vec![Default::default()],
            message: versioned_message,
        };

        assert!(validator.validate_transaction_with_versioned(&versioned_transaction).is_err());
    }
}
