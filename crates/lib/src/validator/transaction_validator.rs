use crate::{
    config::FeePayerPolicy,
    error::KoraError,
    fee::fee::FeeConfigUtil,
    oracle::PriceSource,
    state::get_config,
    token::{
        interface::{SplInstructionCommand, TokenMint},
        token::{TokenType, TokenUtil},
    },
    transaction::VersionedTransactionExt,
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

impl TransactionValidator {
    pub fn new(fee_payer_pubkey: Pubkey) -> Result<Self, KoraError> {
        let config = &get_config()?.validation;

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
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        // First check if the mint is in allowed tokens
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {mint} is not a valid token mint"
            )));
        }

        let mint = TokenUtil::get_mint(rpc_client, mint).await?;

        Ok(mint)
    }

    /// Safe account key resolution for both Legacy and V0 transactions
    fn get_account_key(
        account_keys: &[Pubkey],
        index: u8,
        context: &str,
    ) -> Result<Pubkey, KoraError> {
        let idx = index as usize;
        if idx >= account_keys.len() {
            return Err(KoraError::InvalidTransaction(format!(
                "Account index {idx} out of bounds for {context}. Available accounts: {}",
                account_keys.len()
            )));
        }
        Ok(account_keys[idx])
    }

    /*
    This function is used to validate a transaction.
    It takes a VersionedTransactionResolved or VersionedTransaction.
    The caller is responsible for resolving the addresses before calling this function if needed.
     */
    pub async fn validate_transaction(
        &self,
        rpc_client: &RpcClient,
        transaction_resolved: &impl VersionedTransactionExt,
    ) -> Result<(), KoraError> {
        let transaction = transaction_resolved.get_transaction();
        let all_account_keys = transaction_resolved.get_all_account_keys();

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

        self.validate_signatures(transaction)?;

        self.validate_programs(&transaction.message, &all_account_keys)?;
        self.validate_transfer_amounts(rpc_client, &transaction.message, &all_account_keys).await?;
        self.validate_disallowed_accounts(&transaction.message, &all_account_keys)?;
        self.validate_fee_payer_usage(&transaction.message, &all_account_keys)?;

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

    fn validate_programs(
        &self,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<(), KoraError> {
        for instruction in message.instructions() {
            let program_id =
                Self::get_account_key(account_keys, instruction.program_id_index, "program ID")?;
            if !self.allowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {program_id} is not in the allowed list"
                )));
            }
        }
        Ok(())
    }

    fn validate_fee_payer_usage(
        &self,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<(), KoraError> {
        // Ensure fee payer is not being used as a source of funds
        for instruction in message.instructions() {
            if self.is_fee_payer_source(instruction, account_keys)? {
                return Err(KoraError::InvalidTransaction(
                    "Fee payer cannot be used as source account".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn check_fee_payer(
        fee_payer_pubkey: &Pubkey,
        account_index: usize,
        policy_allowed: bool,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Result<bool, KoraError> {
        if account_index >= ix.accounts.len() {
            return Ok(false); // If account index is invalid, fee payer can't be the source
        }
        let account_key_index = ix.accounts[account_index];
        match Self::get_account_key(account_keys, account_key_index, "fee payer source check") {
            Ok(account_pubkey) => Ok(!policy_allowed && account_pubkey == *fee_payer_pubkey),
            Err(_) => Ok(false), // If account index is invalid, fee payer can't be the source
        }
    }

    fn is_fee_payer_source(
        &self,
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Result<bool, KoraError> {
        let program_id =
            Self::get_account_key(account_keys, ix.program_id_index, "fee payer check")?;

        match program_id {
            SYSTEM_PROGRAM_ID => {
                if let Ok(sys_ix) = bincode::deserialize::<SystemInstruction>(&ix.data) {
                    let (index, policy_allowed) = match sys_ix {
                        SystemInstruction::Transfer { .. } => {
                            (0, self.fee_payer_policy.allow_sol_transfers)
                        }
                        SystemInstruction::TransferWithSeed { .. } => {
                            (1, self.fee_payer_policy.allow_sol_transfers)
                        }
                        SystemInstruction::Assign { .. } => (0, self.fee_payer_policy.allow_assign),
                        SystemInstruction::AssignWithSeed { .. } => {
                            (1, self.fee_payer_policy.allow_assign)
                        }
                        _ => (0, false),
                    };
                    Self::check_fee_payer(
                        &self.fee_payer_pubkey,
                        index,
                        policy_allowed,
                        ix,
                        account_keys,
                    )
                } else {
                    Ok(false)
                }
            }
            spl_token::ID | spl_token_2022::ID => {
                let command = SplInstructionCommand::ValidateFeePayerPolicy {
                    fee_payer_policy: self.fee_payer_policy.clone(),
                };

                let token_interface = TokenType::get_token_program_from_owner(&program_id)?;

                let result = token_interface.process_spl_instruction(
                    &ix.data,
                    ix,
                    account_keys,
                    &self.fee_payer_pubkey,
                    command,
                )?;

                Ok(result)
            }
            _ => Ok(false),
        }
    }

    async fn validate_transfer_amounts(
        &self,
        rpc_client: &RpcClient,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<(), KoraError> {
        let total_outflow = self.calculate_total_outflow(rpc_client, message, account_keys).await?;

        if total_outflow > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                total_outflow, self.max_allowed_lamports
            )));
        }

        Ok(())
    }

    fn validate_disallowed_accounts(
        &self,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<(), KoraError> {
        for instruction in message.instructions() {
            let program_id = Self::get_account_key(
                account_keys,
                instruction.program_id_index,
                "disallowed account check (program)",
            )?;
            if self.disallowed_accounts.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {program_id} is disallowed"
                )));
            }

            for account_index in instruction.accounts.iter() {
                let account_pubkey = Self::get_account_key(
                    account_keys,
                    *account_index,
                    "disallowed account check",
                )?;
                if self.disallowed_accounts.contains(&account_pubkey) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Account {account_pubkey} is disallowed"
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    async fn calculate_total_outflow(
        &self,
        rpc_client: &RpcClient,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<u64, KoraError> {
        // Use the shared outflow calculation from fees module
        FeeConfigUtil::calculate_fee_payer_outflow(
            rpc_client,
            &self.fee_payer_pubkey,
            message,
            account_keys,
        )
        .await
    }

    pub async fn validate_token_payment(
        // Should have resolved addresses for lookup tables
        resolved_transaction: &impl VersionedTransactionExt,
        required_lamports: u64,
        rpc_client: &RpcClient,
        expected_payment_destination: &Pubkey,
    ) -> Result<(), KoraError> {
        let mut total_lamport_value = 0;

        let transaction = resolved_transaction.get_transaction();
        let all_account_keys = resolved_transaction.get_all_account_keys();

        for ix in transaction.message.instructions() {
            // Safe program ID resolution
            let program_idx = ix.program_id_index as usize;
            if program_idx >= all_account_keys.len() {
                continue; // Skip invalid program ID index
            }
            let program_id = all_account_keys[program_idx];

            if let Ok(token_program) = TokenType::get_token_program_from_owner(&program_id) {
                if TokenUtil::process_token_transfer(
                    ix,
                    &*token_program,
                    &all_account_keys,
                    rpc_client,
                    &mut total_lamport_value,
                    required_lamports,
                    expected_payment_destination,
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
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{Config, KoraConfig, MetricsConfig, ValidationConfig},
        state::update_config,
        tests::common::get_mock_rpc_client,
        transaction::TransactionUtil,
    };
    use serial_test::serial;

    use super::*;
    use solana_message::Message;
    use solana_sdk::{account::Account, instruction::Instruction};
    use solana_system_interface::instruction::{
        assign, create_account, create_account_with_seed, transfer, transfer_with_seed,
    };

    #[tokio::test]
    #[serial]
    async fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let instruction = transfer(&sender, &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_transfer_amount_limits() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test transaction with amount over limit
        let instruction = transfer(&sender, &recipient, 2_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok()); // Should pass because sender is not fee payer

        // Test multiple transfers
        let instructions =
            vec![transfer(&sender, &recipient, 500_000), transfer(&sender, &recipient, 500_000)];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_programs() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()]) // System program
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test allowed program (system program)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test disallowed program
        let fake_program = Pubkey::new_unique();
        // Create a no-op instruction for the fake program
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_signatures() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_max_signatures(2)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test too many signatures
        let instructions = vec![
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        transaction.signatures = vec![Default::default(); 3]; // Add 3 dummy signatures
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_sign_and_send_transaction_mode() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test SignAndSend mode with fee payer already set should not error
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test SignAndSend mode without fee payer (should succeed)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], None)); // No fee payer specified
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_transaction() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        // Create an empty message using Message::new with empty instructions
        let message = VersionedMessage::Legacy(Message::new(&[], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_accounts() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_disallowed_accounts(vec![
                "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()
            ])
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let instruction = transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_sol_transfers() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test with allow_sol_transfers = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_sol_transfers = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy {
                allow_sol_transfers: false,
                ..Default::default()
            });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_assign() {
        let fee_payer = Pubkey::new_unique();
        let new_owner = Pubkey::new_unique();

        // Test with allow_assign = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_assign = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy { allow_assign: false, ..Default::default() });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();

        // Test with allow_spl_transfers = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_spl_transfers = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy {
                allow_spl_transfers: false,
                ..Default::default()
            });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_token2022_transfers = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_token2022_transfers = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy {
                allow_token2022_transfers: false,
                ..Default::default()
            });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should fail because fee payer is not allowed to be source
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());

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
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should pass because fee payer is not the source
        let rpc_client = get_mock_rpc_client(&Account::default());
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_calculate_total_outflow() {
        let fee_payer = Pubkey::new_unique();
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test 1: Fee payer as sender in Transfer - should add to outflow
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient in Transfer - should subtract from outflow (account closure)
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));

        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
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
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
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
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
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
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        // Test 6: Multiple instructions - should sum correctly
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100_000
            transfer(&sender, &fee_payer, 30_000),     // -30_000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50_000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );

        // Test 7: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");

        // Test 8: Other account funding CreateAccount - should not affect outflow
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let outflow = validator
            .calculate_total_outflow(&rpc_client, &message, message.static_account_keys())
            .await
            .unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_burn = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let burn_ix = spl_token::instruction::burn(
            &spl_token::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should pass because allow_burn is true by default
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_burn = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy { allow_burn: false, ..Default::default() });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let burn_ix = spl_token::instruction::burn(
            &spl_token::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should fail because fee payer cannot burn tokens when allow_burn is false
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());

        // Test burn_checked instruction
        let burn_checked_ix = spl_token::instruction::burn_checked(
            &spl_token::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_checked_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should also fail for burn_checked
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let close_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should pass because allow_close_account is true by default
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_close_account = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy {
                allow_close_account: false,
                ..Default::default()
            });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let close_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should fail because fee payer cannot close accounts when allow_close_account is false
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let approve_ix = spl_token::instruction::approve(
            &spl_token::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should pass because allow_approve is true by default
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_approve = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy { allow_approve: false, ..Default::default() });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let approve_ix = spl_token::instruction::approve(
            &spl_token::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());

        // Test approve_checked instruction
        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token::instruction::approve_checked(
            &spl_token::id(),
            &fee_payer_token_account,
            &mint,
            &delegate,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message =
            VersionedMessage::Legacy(Message::new(&[approve_checked_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should also fail for approve_checked
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_burn = false for Token2022
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy { allow_burn: false, ..Default::default() });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let burn_ix = spl_token_2022::instruction::burn(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should fail for Token2022 burn
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = false for Token2022
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy {
                allow_close_account: false,
                ..Default::default()
            });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let close_ix = spl_token_2022::instruction::close_account(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should fail for Token2022 close account
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true (default)
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default());

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let approve_ix = spl_token_2022::instruction::approve(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let rpc_client = get_mock_rpc_client(&Account::default());
        // Should pass because allow_approve is true by default
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_ok());

        // Test with allow_approve = false
        let validation_config = ValidationConfig::test_default()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy { allow_approve: false, ..Default::default() });

        let config = Config {
            validation: validation_config,
            kora: KoraConfig::default(),
            metrics: MetricsConfig::default(),
        };
        update_config(config).unwrap();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        let approve_ix = spl_token_2022::instruction::approve(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());

        // Test approve_checked instruction
        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token_2022::instruction::approve_checked(
            &spl_token_2022::id(),
            &fee_payer_token_account,
            &mint,
            &delegate,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message =
            VersionedMessage::Legacy(Message::new(&[approve_checked_ix], Some(&fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        // Should also fail for approve_checked
        assert!(validator.validate_transaction(&rpc_client, &transaction).await.is_err());
    }
}
