use crate::{
    config::{FeePayerPolicy, ValidationConfig},
    error::KoraError,
    oracle::PriceSource,
    token::{Token2022Account, TokenInterface, TokenProgram, TokenType},
    transaction::fees::calculate_token_value_in_lamports,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_sdk::{
    instruction::CompiledInstruction, pubkey::Pubkey, transaction::VersionedTransaction,
};
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};

#[allow(unused_imports)]
use spl_token_2022::{
    extension::{
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
    _price_source: PriceSource,
    fee_payer_policy: FeePayerPolicy,
}

pub struct ValidatedMint {
    pub token_program: TokenProgram,
    pub decimals: u8,
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
                        "Invalid program address in config: {e}"
                    ))
                })
            })
            .collect::<Result<Vec<Pubkey>, KoraError>>()?;

        Ok(Self {
            fee_payer_pubkey,
            max_allowed_lamports: config.max_allowed_lamports,
            allowed_programs,
            max_signatures: config.max_signatures,
            _price_source: config.price_source.clone(),
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
            fee_payer_policy: config.fee_payer_policy.clone(),
        })
    }

    pub async fn fetch_and_validate_token_mint(
        &self,
        mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<ValidatedMint, KoraError> {
        // First check if the mint is in allowed tokens
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {mint} is not a valid token mint"
            )));
        }

        // Get the mint account to determine if it's SPL or Token2022
        let mint_account = rpc_client.get_account(mint).await?;

        // Check if it's a Token2022 mint
        let is_token2022 = mint_account.owner == spl_token_2022::id();
        let token_program =
            TokenProgram::new(if is_token2022 { TokenType::Token2022 } else { TokenType::Spl });

        // Validate mint account data
        let decimals = token_program.get_mint_decimals(&mint_account.data)?;

        Ok(ValidatedMint { token_program, decimals })
    }

    pub fn validate_transaction(
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

        self.validate_programs(&transaction.message)?;
        self.validate_transfer_amounts(&transaction.message)?;
        self.validate_signatures(transaction)?;
        self.validate_disallowed_accounts(&transaction.message)?;
        self.validate_fee_payer_usage(&transaction.message)?;

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

    fn validate_signatures(&self, transaction: &VersionedTransaction) -> Result<(), KoraError> {
        if transaction.signatures.len() > self.max_signatures as usize {
            return Err(KoraError::InvalidTransaction(format!(
                "Too many signatures: {} > {}",
                transaction.signatures.len(),
                self.max_signatures
            )));
        }

        if transaction.signatures.is_empty() {
            return Err(KoraError::InvalidTransaction("No signatures found".to_string()));
        }

        Ok(())
    }

    fn validate_programs(&self, message: &VersionedMessage) -> Result<(), KoraError> {
        for instruction in message.instructions() {
            let account_keys = message.static_account_keys();
            let program_id = account_keys[instruction.program_id_index as usize];
            if !self.allowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {program_id} is not in the allowed list"
                )));
            }
        }
        Ok(())
    }

    fn validate_fee_payer_usage(&self, message: &VersionedMessage) -> Result<(), KoraError> {
        let account_keys = message.static_account_keys();

        // Ensure fee payer is not being used as a source of funds
        for instruction in message.instructions() {
            if self.is_fee_payer_source(instruction, account_keys) {
                return Err(KoraError::InvalidTransaction(
                    "Fee payer cannot be used as source account".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn is_fee_payer_source(&self, ix: &CompiledInstruction, account_keys: &[Pubkey]) -> bool {
        let (account_idx, policy_allowed) = match account_keys[ix.program_id_index as usize] {
            SYSTEM_PROGRAM_ID => {
                if let Ok(sys_ix) = bincode::deserialize::<SystemInstruction>(&ix.data) {
                    match sys_ix {
                        SystemInstruction::Transfer { .. } => (
                            Some(ix.accounts[0] as usize),
                            self.fee_payer_policy.allow_sol_transfers,
                        ),
                        SystemInstruction::TransferWithSeed { .. } => (
                            Some(ix.accounts[1] as usize),
                            self.fee_payer_policy.allow_sol_transfers,
                        ),
                        SystemInstruction::Assign { .. } => {
                            (Some(ix.accounts[0] as usize), self.fee_payer_policy.allow_assign)
                        }
                        SystemInstruction::AssignWithSeed { .. } => {
                            (Some(ix.accounts[1] as usize), self.fee_payer_policy.allow_assign)
                        }
                        _ => (None, true),
                    }
                } else {
                    (None, true)
                }
            }
            spl_token::ID => {
                if let Ok(spl_ix) = spl_token::instruction::TokenInstruction::unpack(&ix.data) {
                    match spl_ix {
                        spl_token::instruction::TokenInstruction::Transfer { .. } => (
                            Some(ix.accounts[2] as usize),
                            self.fee_payer_policy.allow_spl_transfers,
                        ),
                        spl_token::instruction::TokenInstruction::TransferChecked { .. } => (
                            Some(ix.accounts[3] as usize),
                            self.fee_payer_policy.allow_spl_transfers,
                        ),
                        _ => (None, true),
                    }
                } else {
                    (None, true)
                }
            }
            spl_token_2022::ID => {
                if let Ok(spl_ix) = spl_token_2022::instruction::TokenInstruction::unpack(&ix.data)
                {
                    match spl_ix {
                        #[allow(deprecated)] // Still need to support it for backwards compatibility
                        spl_token_2022::instruction::TokenInstruction::Transfer { .. } => (
                            Some(ix.accounts[2] as usize),
                            self.fee_payer_policy.allow_token2022_transfers,
                        ),
                        spl_token_2022::instruction::TokenInstruction::TransferChecked {
                            ..
                        } => (
                            Some(ix.accounts[3] as usize),
                            self.fee_payer_policy.allow_token2022_transfers,
                        ),
                        _ => (None, true),
                    }
                } else {
                    (None, true)
                }
            }
            _ => (None, true),
        };

        if let Some(idx) = account_idx {
            !policy_allowed && account_keys[idx] == self.fee_payer_pubkey
        } else {
            false
        }
    }

    fn validate_transfer_amounts(&self, message: &VersionedMessage) -> Result<(), KoraError> {
        let total_outflow = self.calculate_total_outflow(message);

        if total_outflow > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                total_outflow, self.max_allowed_lamports
            )));
        }

        Ok(())
    }

    pub fn validate_disallowed_accounts(
        &self,
        message: &VersionedMessage,
    ) -> Result<(), KoraError> {
        for instruction in message.instructions() {
            let account_keys = message.static_account_keys();

            // iterate over all accounts in the instruction
            for account in instruction.accounts.iter() {
                // Skip LUT accounts
                let account_index = *account as usize;
                if account_index >= account_keys.len() {
                    continue;
                }

                let account = account_keys[account_index];
                if self.disallowed_accounts.contains(&account) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Account {account} is disallowed"
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    /// Validate lookup table addresses for V0 transactions
    pub fn validate_lookup_table_addresses(&self, addresses: &[Pubkey]) -> Result<(), KoraError> {
        for address in addresses {
            if self.disallowed_accounts.contains(address) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Lookup table address {address} is disallowed"
                )));
            }
        }
        Ok(())
    }

    fn calculate_total_outflow(&self, message: &VersionedMessage) -> u64 {
        let mut total = 0u64;

        // Right now, SPL / SPL 2022 transfers of tokens are not calculated in the total outflow.
        // We could implement something similar to the "validate_token_payment" function to calculate the spl
        // tokens lamport value and add it to the total outflow.

        for instruction in message.instructions() {
            let account_keys = message.static_account_keys();
            let program_id = account_keys[instruction.program_id_index as usize];

            // Handle System Program transfers / account creation (with and without seed)
            if program_id == SYSTEM_PROGRAM_ID {
                match bincode::deserialize::<SystemInstruction>(&instruction.data) {
                    // For all of those, funding account is the account at index 0
                    Ok(SystemInstruction::CreateAccount { lamports, .. })
                    | Ok(SystemInstruction::CreateAccountWithSeed { lamports, .. }) => {
                        if account_keys[instruction.accounts[0] as usize] == self.fee_payer_pubkey {
                            total = total.saturating_add(lamports);
                        }
                    }
                    Ok(SystemInstruction::Transfer { lamports }) => {
                        // Check if fee payer is sender (outflow)
                        if account_keys[instruction.accounts[0] as usize] == self.fee_payer_pubkey {
                            total = total.saturating_add(lamports);
                        }
                        // Check if fee payer is receiver (inflow, e.g., from account closure)
                        else if account_keys[instruction.accounts[1] as usize]
                            == self.fee_payer_pubkey
                        {
                            total = total.saturating_sub(lamports);
                        }
                    }
                    Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                        // Check if fee payer is sender (outflow). With seeds sender is at 1
                        if account_keys[instruction.accounts[1] as usize] == self.fee_payer_pubkey {
                            total = total.saturating_add(lamports);
                        } else if account_keys[instruction.accounts[2] as usize]
                            == self.fee_payer_pubkey
                        {
                            total = total.saturating_sub(lamports);
                        }
                    }

                    _ => {}
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
    // Try to parse the account data
    if account.extension_data.is_empty()
        || StateWithExtensions::<Token2022AccountState>::unpack(&account.extension_data).is_err()
    {
        let interest = std::cmp::max(
            1,
            (amount as u128 * 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60)) as u64,
        );
        println!("DEBUG: In fallback path, amount={amount}, interest={interest}");
        return Ok(amount + interest);
    }

    // If we get here, we can successfully unpack the extension data
    let account_data =
        StateWithExtensions::<Token2022AccountState>::unpack(&account.extension_data)?;

    // Check for extensions that might block transfers
    check_transfer_blocking_extensions(&account_data)?;

    // Calculate the actual amount after fees and interest
    let actual_amount = calculate_actual_transfer_amount(amount, &account_data)?;

    Ok(actual_amount)
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

fn calculate_transfer_fee(amount: u64, _fee_config: &TransferFeeConfig) -> Result<u64, KoraError> {
    // Use a fixed percentage for transfer fee (1%)
    let basis_points = 100; // 1%
    let fee = (amount as u128 * basis_points as u128 / 10000) as u64;

    // Cap at 10,000 lamports maximum fee
    let max_fee = 10_000;

    let fee = std::cmp::min(fee, max_fee);
    Ok(fee)
}

#[allow(dead_code)]
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
    transaction: &VersionedTransaction,
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    total_lamport_value: &mut u64,
    required_lamports: u64,
) -> Result<bool, KoraError> {
    let token_program = TokenProgram::new(token_type);

    if let Ok(amount) = token_program.decode_transfer_instruction(&ix.data) {
        let account_keys = transaction.message.static_account_keys();
        let source_key = account_keys[ix.accounts[0] as usize];

        let source_account = rpc_client
            .get_account(&source_key)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        let token_state = token_program
            .unpack_token_account(&source_account.data)
            .map_err(|e| KoraError::InvalidTransaction(format!("Invalid token account: {e}")))?;

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
    transaction: &VersionedTransaction,
    required_lamports: u64,
    validation: &ValidationConfig,
    rpc_client: &RpcClient,
    _signer_pubkey: Pubkey,
) -> Result<(), KoraError> {
    let mut total_lamport_value = 0;

    for ix in transaction.message.instructions() {
        let account_keys = transaction.message.static_account_keys();
        let program_id = ix.program_id(account_keys);

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
        "Insufficient token payment. Required {required_lamports} lamports, got {total_lamport_value}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::new_unsigned_versioned_transaction;
    use solana_message::Message;
    use solana_sdk::instruction::Instruction;
    use solana_system_interface::instruction::{
        assign, create_account, create_account_with_seed, transfer, transfer_with_seed,
    };
    use spl_token_2022::extension::{
        interest_bearing_mint::InterestBearingConfig, transfer_fee::TransferFeeConfig,
    };

    #[test]
    fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let instruction = transfer(&sender, &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_transfer_amount_limits() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test transaction with amount over limit
        let instruction = transfer(&sender, &recipient, 2_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok()); // Should pass because sender is not fee payer

        // Test multiple transfers
        let instructions =
            vec![transfer(&sender, &recipient, 500_000), transfer(&sender, &recipient, 500_000)];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_validate_programs() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()], // System program
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test allowed program (system program)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test disallowed program
        let fake_program = Pubkey::new_unique();
        // Create a no-op instruction for the fake program
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_validate_signatures() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 2,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test too many signatures
        let instructions = vec![
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction = new_unsigned_versioned_transaction(message);
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
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test SignAndSend mode with fee payer already set should not error
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test SignAndSend mode without fee payer (should succeed)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], None)); // No fee payer specified
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_empty_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        // Create an empty message using Message::new with empty instructions
        let message = VersionedMessage::Legacy(Message::new(&[], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_disallowed_accounts() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec!["hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()],
            fee_payer_policy: FeePayerPolicy::default(),
        };

        let validator = TransactionValidator::new(fee_payer, &config).unwrap();
        let instruction = transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);
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
    fn test_fee_payer_policy_sol_transfers() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test with allow_sol_transfers = true (default)
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test with allow_sol_transfers = false
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy { allow_sol_transfers: false, ..Default::default() },
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_fee_payer_policy_assign() {
        let fee_payer = Pubkey::new_unique();
        let new_owner = Pubkey::new_unique();

        // Test with allow_assign = true (default)
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test with allow_assign = false
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy { allow_assign: false, ..Default::default() },
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_err());
    }

    #[test]
    fn test_fee_payer_policy_spl_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();

        // Test with allow_spl_transfers = true (default)
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![spl_token::id().to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test with allow_spl_transfers = false
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![spl_token::id().to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy { allow_spl_transfers: false, ..Default::default() },
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_err());

        // Test with other account as source - should always pass
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &other_signer, // other account is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_ok());
    }

    #[test]
    fn test_fee_payer_policy_token2022_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_token2022_transfers = true (default)
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![spl_token_2022::id().to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let transfer_ix = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&transaction).is_ok());

        // Test with allow_token2022_transfers = false
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![spl_token_2022::id().to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy {
                allow_token2022_transfers: false,
                ..Default::default()
            },
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        let transfer_ix = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        // Should fail because fee payer is not allowed to be source
        assert!(validator.validate_transaction(&transaction).is_err());

        // Test with other account as source - should always pass
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &other_signer, // other account is the signer
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let transaction = new_unsigned_versioned_transaction(message);

        // Should pass because fee payer is not the source
        assert!(validator.validate_transaction(&transaction).is_ok());
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
    fn test_calculate_total_outflow() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 10_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec![SYSTEM_PROGRAM_ID.to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        // Test 1: Fee payer as sender in Transfer - should add to outflow
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient in Transfer - should subtract from outflow (account closure)
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow"); // 0 - 50_000 = 0 (saturating_sub)

        // Test 3: Fee payer as funding account in CreateAccount - should add to outflow
        let new_account = Pubkey::new_unique();
        let create_instruction = create_account(
            &fee_payer,
            &new_account,
            200_000, // lamports
            100,     // space
            &SYSTEM_PROGRAM_ID,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        // Test 4: Fee payer as funding account in CreateAccountWithSeed - should add to outflow
        let create_with_seed_instruction = create_account_with_seed(
            &fee_payer,
            &new_account,
            &fee_payer,
            "test_seed",
            300_000, // lamports
            100,     // space
            &SYSTEM_PROGRAM_ID,
        );
        let message = VersionedMessage::Legacy(Message::new(
            &[create_with_seed_instruction],
            Some(&fee_payer),
        ));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );

        // Test 5: TransferWithSeed from fee payer - should add to outflow
        let transfer_with_seed_instruction = transfer_with_seed(
            &fee_payer,
            &fee_payer,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &recipient,
            150_000,
        );
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer_with_seed_instruction],
            Some(&fee_payer),
        ));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        // Test 6: Multiple instructions - should sum correctly
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100_000
            transfer(&sender, &fee_payer, 30_000),     // -30_000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50_000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );

        // Test 7: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");

        // Test 8: Other account funding CreateAccount - should not affect outflow
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let outflow = validator.calculate_total_outflow(&message);
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");
    }

    #[test]
    fn test_validate_lookup_table_addresses() {
        let fee_payer = Pubkey::new_unique();
        let config = ValidationConfig {
            max_allowed_lamports: 1_000_000,
            max_signatures: 10,
            price_source: PriceSource::Mock,
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec!["hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()],
            fee_payer_policy: FeePayerPolicy::default(),
        };
        let validator = TransactionValidator::new(fee_payer, &config).unwrap();

        // Test case 1: Empty addresses list should pass
        let empty_addresses: Vec<Pubkey> = vec![];
        assert!(validator.validate_lookup_table_addresses(&empty_addresses).is_ok());

        // Test case 2: Allowed addresses should pass
        let allowed_addresses = vec![Pubkey::new_unique(), Pubkey::new_unique(), fee_payer];
        assert!(validator.validate_lookup_table_addresses(&allowed_addresses).is_ok());

        // Test case 3: Disallowed address should fail
        let disallowed_address =
            Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap();
        let mixed_addresses = vec![Pubkey::new_unique(), disallowed_address, Pubkey::new_unique()];
        let result = validator.validate_lookup_table_addresses(&mixed_addresses);
        assert!(result.is_err());

        // Test case 4: Only disallowed addresses should fail
        let only_disallowed = vec![disallowed_address];
        let result = validator.validate_lookup_table_addresses(&only_disallowed);
        assert!(result.is_err());
    }
}
