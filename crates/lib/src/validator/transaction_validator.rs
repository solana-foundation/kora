use crate::{
    config::{Config, FeePayerPolicy, ProgramsConfig},
    error::KoraError,
    fee::fee::{FeeConfigUtil, TotalFeeCalculation, TransactionFeeUtil},
    token::{interface::TokenMint, token::TokenUtil},
    transaction::{
        ParsedSPLInstructionData, ParsedSystemInstructionData, VersionedTransactionResolved,
    },
    validator::parse_pubkey_set,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::collections::HashSet;

use crate::fee::price::PriceModel;

mod alt;
mod bpf_loader_upgradeable;
mod loader_v4;
mod spl_token;
mod system;
mod token_2022;

pub struct TransactionValidator {
    fee_payer_pubkey: Pubkey,
    max_allowed_lamports: u64,
    max_priority_fee_lamports: Option<u64>,
    allowed_programs: HashSet<Pubkey>,
    allow_all_programs: bool,
    require_one_of_programs: HashSet<Pubkey>,
    max_signatures: u64,
    allowed_tokens: HashSet<Pubkey>,
    disallowed_accounts: HashSet<Pubkey>,
    fee_payer_policy: FeePayerPolicy,
    allow_durable_transactions: bool,
}

impl TransactionValidator {
    pub fn new(config: &Config, fee_payer_pubkey: Pubkey) -> Result<Self, KoraError> {
        let config = &config.validation;

        let (allow_all_programs, allowed_programs) = match &config.allowed_programs {
            ProgramsConfig::All => (true, HashSet::new()),
            ProgramsConfig::Allowlist(programs) => (false, parse_pubkey_set(programs)?),
        };

        Ok(Self {
            fee_payer_pubkey,
            max_allowed_lamports: config.max_allowed_lamports,
            max_priority_fee_lamports: config.max_priority_fee_lamports,
            allowed_programs,
            allow_all_programs,
            require_one_of_programs: parse_pubkey_set(&config.require_one_of_programs)?,
            max_signatures: config.max_signatures,
            allowed_tokens: parse_pubkey_set(&config.allowed_tokens)?,
            disallowed_accounts: parse_pubkey_set(&config.disallowed_accounts)?,
            fee_payer_policy: config.fee_payer_policy.clone(),
            allow_durable_transactions: config.allow_durable_transactions,
        })
    }

    pub async fn fetch_and_validate_token_mint(
        &self,
        mint: &Pubkey,
        config: &Config,
        rpc_client: &RpcClient,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {mint} is not a valid token mint"
            )));
        }

        let mint = TokenUtil::get_mint(config, rpc_client, mint).await?;

        Ok(mint)
    }

    /*
    This function is used to validate a transaction.
     */
    pub async fn validate_transaction(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        if transaction_resolved.all_instructions.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no instructions".to_string(),
            ));
        }

        self.validate_has_non_compute_instruction(transaction_resolved)?;
        Self::warn_on_ignored_compute_budget_instructions(transaction_resolved);

        if transaction_resolved.all_account_keys.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no account keys".to_string(),
            ));
        }

        self.validate_signatures(&transaction_resolved.transaction)?;

        self.validate_programs(transaction_resolved)?;
        self.validate_require_one_of_programs(transaction_resolved)?;
        self.validate_priority_fee(transaction_resolved)?;
        self.validate_transfer_amounts(config, transaction_resolved, rpc_client).await?;
        self.validate_disallowed_accounts(transaction_resolved)?;
        self.validate_fee_payer_usage(config, transaction_resolved)?;

        Ok(())
    }

    fn validate_has_non_compute_instruction(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        let compute_budget_program_id = solana_compute_budget_interface::id();
        let has_non_compute_instruction = transaction_resolved
            .all_instructions
            .iter()
            .any(|ix| ix.program_id != compute_budget_program_id);

        if !has_non_compute_instruction {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains only ComputeBudget instructions".to_string(),
            ));
        }

        Ok(())
    }

    /// ComputeBudget instructions are neither parsed nor rejected in a V1 transaction: they
    /// execute successfully while doing nothing, so a client that sets its resource limits
    /// there silently gets the transaction config's limits instead of the ones it asked for.
    fn warn_on_ignored_compute_budget_instructions(
        transaction_resolved: &VersionedTransactionResolved,
    ) {
        if !matches!(transaction_resolved.transaction.message, VersionedMessage::V1(_)) {
            return;
        }

        let compute_budget_program_id = solana_compute_budget_interface::id();
        if transaction_resolved
            .all_instructions
            .iter()
            .any(|ix| ix.program_id == compute_budget_program_id)
        {
            log::warn!(
                "V1 transaction contains ComputeBudget instructions, which the runtime ignores; \
                 its resource limits come from the transaction config"
            );
        }
    }

    fn validate_priority_fee(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        let Some(max_priority_fee_lamports) = self.max_priority_fee_lamports else {
            return Ok(());
        };

        let priority_fee = TransactionFeeUtil::get_requested_priority_fee(
            &transaction_resolved.transaction.message,
            &transaction_resolved.all_account_keys,
        );
        if priority_fee > max_priority_fee_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Priority fee {priority_fee} exceeds maximum allowed {max_priority_fee_lamports}"
            )));
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
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        if self.allow_all_programs {
            return Ok(());
        }
        for instruction in &transaction_resolved.all_instructions {
            if !self.allowed_programs.contains(&instruction.program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is not in the allowed list",
                    instruction.program_id
                )));
            }
        }
        Ok(())
    }

    fn validate_require_one_of_programs(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        if self.require_one_of_programs.is_empty() {
            return Ok(());
        }

        let called = transaction_resolved
            .all_instructions
            .iter()
            .any(|ix| self.require_one_of_programs.contains(&ix.program_id));
        if !called {
            return Err(KoraError::InvalidTransaction(format!(
                "Transaction must call at least one of the required programs: {:?}",
                self.require_one_of_programs
            )));
        }

        Ok(())
    }

    fn fee_payer_signs(&self, authority: &Pubkey, multisig_signers: &[Pubkey]) -> bool {
        *authority == self.fee_payer_pubkey || multisig_signers.contains(&self.fee_payer_pubkey)
    }

    fn validate_fee_payer_usage(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        self.validate_ata_create_instructions(transaction_resolved)?;

        self.validate_system_fee_payer_usage(
            transaction_resolved.get_or_parse_system_instructions()?,
        )?;
        self.validate_spl_token_fee_payer_usage(
            transaction_resolved.get_or_parse_spl_instructions()?,
        )?;
        self.validate_alt_fee_payer_usage(transaction_resolved.get_or_parse_alt_instructions()?)?;
        self.validate_loader_v4_fee_payer_usage(
            transaction_resolved.get_or_parse_loader_v4_instructions()?,
        )?;
        self.validate_bpf_loader_upgradeable_fee_payer_usage(
            transaction_resolved.get_or_parse_bpf_loader_upgradeable_instructions()?,
        )?;
        self.validate_token2022_fee_payer_usage(
            transaction_resolved.get_or_parse_spl_instructions()?,
        )?;
        self.validate_token2022_extension_security(config, transaction_resolved)?;

        Ok(())
    }

    fn validate_ata_create_instructions(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        if self.fee_payer_policy.system.allow_create_account {
            return Ok(());
        }

        let has_fee_payer_ata_create = !TokenUtil::find_fee_payer_ata_creations(
            &transaction_resolved.all_instructions,
            &self.fee_payer_pubkey,
        )
        .is_empty();

        if has_fee_payer_ata_create {
            return Err(KoraError::InvalidTransaction(
                "Fee payer cannot fund ATA creation (Create or CreateIdempotent)".to_string(),
            ));
        }

        Ok(())
    }

    async fn validate_transfer_amounts(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        let total_outflow =
            self.calculate_total_outflow(config, transaction_resolved, rpc_client).await?;

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
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        for instruction in &transaction_resolved.all_instructions {
            if self.disallowed_accounts.contains(&instruction.program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is disallowed",
                    instruction.program_id
                )));
            }

            for account_index in instruction.accounts.iter() {
                if self.disallowed_accounts.contains(&account_index.pubkey) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Account {} is disallowed",
                        account_index.pubkey
                    )));
                }
            }
        }
        // Validate instruction-data pubkeys that are not present in account metas.
        let system_instructions = transaction_resolved.get_or_parse_system_instructions()?;
        for instruction in system_instructions.values().flatten() {
            match instruction {
                ParsedSystemInstructionData::SystemAuthorizeNonceAccount {
                    new_authority, ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        new_authority,
                        "System AuthorizeNonceAccount new_authority",
                    )?;
                }
                ParsedSystemInstructionData::SystemInitializeNonceAccount {
                    nonce_authority,
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        nonce_authority,
                        "System InitializeNonceAccount nonce_authority",
                    )?;
                }
                _ => {}
            }
        }

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;
        for instruction in spl_instructions.values().flatten() {
            match instruction {
                ParsedSPLInstructionData::SplTokenSetAuthority {
                    new_authority: Some(new_authority),
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        new_authority,
                        "SPL/Token2022 SetAuthority new_authority",
                    )?;
                }
                ParsedSPLInstructionData::SplTokenInitializeAccount { owner, .. } => {
                    self.validate_disallowed_instruction_data_account(
                        owner,
                        "SPL/Token2022 InitializeAccount owner",
                    )?;
                }
                ParsedSPLInstructionData::SplTokenInitializeMint {
                    mint_authority,
                    freeze_authority,
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        mint_authority,
                        "SPL/Token2022 InitializeMint mint_authority",
                    )?;
                    if let Some(freeze_authority) = freeze_authority {
                        self.validate_disallowed_instruction_data_account(
                            freeze_authority,
                            "SPL/Token2022 InitializeMint freeze_authority",
                        )?;
                    }
                }
                _ => {}
            }
        }

        for instruction in transaction_resolved.get_or_parse_token2022_security_instructions()? {
            for field in &instruction.data_pubkeys {
                self.validate_disallowed_instruction_data_account(&field.pubkey, field.context)?;
            }
        }

        Ok(())
    }

    fn validate_disallowed_instruction_data_account(
        &self,
        account: &Pubkey,
        context: &str,
    ) -> Result<(), KoraError> {
        if self.is_disallowed_account(account) {
            return Err(KoraError::InvalidTransaction(format!(
                "Disallowed account {} found in instruction data for {}",
                account, context
            )));
        }
        Ok(())
    }

    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    async fn calculate_total_outflow(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let net = FeeConfigUtil::calculate_fee_payer_outflow(
            &self.fee_payer_pubkey,
            transaction_resolved,
            rpc_client,
            config,
        )
        .await?;
        Ok(net.max(0) as u64)
    }

    pub async fn validate_token_payment(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        required_lamports: u64,
        rpc_client: &RpcClient,
        expected_payment_destination: &Pubkey,
    ) -> Result<(), KoraError> {
        if TokenUtil::verify_token_payment(
            config,
            transaction_resolved,
            rpc_client,
            required_lamports,
            expected_payment_destination,
            None,
        )
        .await?
        {
            return Ok(());
        }

        Err(KoraError::InvalidTransaction(format!(
            "Insufficient token payment. Required {required_lamports} lamports"
        )))
    }

    pub fn validate_strict_pricing_with_fee(
        config: &Config,
        fee_calculation: &TotalFeeCalculation,
    ) -> Result<(), KoraError> {
        if !matches!(&config.validation.price.model, PriceModel::Fixed { strict: true, .. }) {
            return Ok(());
        }

        let fixed_price_lamports = fee_calculation.total_fee_lamports;
        let total_fee_lamports = fee_calculation.get_total_fee_lamports()?;

        if fixed_price_lamports < total_fee_lamports {
            log::error!(
                "Strict pricing violation: fixed_price_lamports={} < total_fee_lamports={}",
                fixed_price_lamports,
                total_fee_lamports
            );
            return Err(KoraError::ValidationError(format!(
                    "Strict pricing violation: total fee ({} lamports) exceeds fixed price ({} lamports)",
                    total_fee_lamports,
                    fixed_price_lamports
                )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{Config, FeePayerPolicy, TransferHookPolicy},
        oracle::PriceSource,
        state::{get_config, update_config},
        tests::{
            account_mock::{AccountMockBuilder, MintAccountMockBuilder, TokenAccountMockBuilder},
            config_mock::{mock_state::setup_config_mock, ConfigMockBuilder},
            rpc_mock::RpcMockBuilder,
            transaction_mock::{
                create_legacy_message, create_resolved_with_loaded_keys,
                create_v0_message_with_alt_loaded_program, create_v1_message,
                create_v1_message_with_instructions,
            },
        },
        token::token::TransferHookValidationFlow,
        transaction::TransactionUtil,
    };
    use serial_test::serial;
    use solana_nullable::MaybeNull;
    use std::str::FromStr;

    use super::*;
    use crate::constant::instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR;
    use solana_address_lookup_table_interface::{
        instruction as alt_instruction, program::ID as ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
    };
    use solana_compute_budget_interface::ComputeBudgetInstruction;
    use solana_message::{v1, Message, VersionedMessage};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
    };
    use solana_system_interface::{
        instruction::{
            assign, create_account, create_account_with_seed, transfer, transfer_with_seed,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };

    fn setup_both_configs(config: Config) {
        drop(setup_config_mock(config.clone()));
        update_config(config).unwrap();
    }

    fn setup_default_config() {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        setup_both_configs(config);
    }

    fn setup_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_spl_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_token2022_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_token2022_config_confidential_allowed(policy: FeePayerPolicy) {
        let mut config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        config.validation.token_2022.allow_confidential_transfers = true;
        setup_both_configs(config);
    }

    fn setup_token2022_config_interface_allowed(policy: FeePayerPolicy) {
        let mut config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        config.validation.token_2022.allow_token_metadata_instructions = true;
        config.validation.token_2022.allow_token_group_instructions = true;
        setup_both_configs(config);
    }

    fn setup_config_with_policy_and_disallowed(
        policy: FeePayerPolicy,
        allowed_programs: Vec<String>,
        disallowed_accounts: Vec<String>,
    ) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(allowed_programs)
            .with_disallowed_accounts(disallowed_accounts)
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_alt_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let instruction = transfer(&sender, &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_transfer_amount_limits() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 2_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let instructions =
            vec![transfer(&sender, &recipient, 500_000), transfer(&sender, &recipient, 500_000)];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_programs() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let fake_program = Pubkey::new_unique();
        let instruction = Instruction::new_with_bincode(fake_program, &[0u8], vec![]);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_programs_wildcard_sentinel() {
        let fee_payer = Pubkey::new_unique();
        let mut config = ConfigMockBuilder::new().with_price_source(PriceSource::Mock).build();
        config.validation.allowed_programs = ProgramsConfig::All;
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let arbitrary_program = Pubkey::new_unique();
        let instruction = Instruction::new_with_bincode(arbitrary_program, &[0u8], vec![]);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_empty_no_restriction() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_require_one_of_programs(vec![])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_only_cu_blocked() {
        let fee_payer = Pubkey::new_unique();
        let compute_budget_id = solana_compute_budget_interface::id().to_string();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![compute_budget_id.clone(), system_id.clone()])
            .with_require_one_of_programs(vec![system_id])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let cu_ix =
            Instruction::new_with_bincode(solana_compute_budget_interface::id(), &[0u8], vec![]);
        let message = VersionedMessage::Legacy(Message::new(&[cu_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_required_program_called() {
        let fee_payer = Pubkey::new_unique();
        let compute_budget_id = solana_compute_budget_interface::id().to_string();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![compute_budget_id.clone(), system_id.clone()])
            .with_require_one_of_programs(vec![system_id])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let cu_ix =
            Instruction::new_with_bincode(solana_compute_budget_interface::id(), &[0u8], vec![]);
        let transfer_ix = transfer(&sender, &recipient, 1000);
        let message =
            VersionedMessage::Legacy(Message::new(&[cu_ix, transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_no_required_program_fails() {
        let fee_payer = Pubkey::new_unique();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let other_program = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![system_id.clone()])
            .with_require_one_of_programs(vec![other_program.to_string()])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_or_semantics() {
        let fee_payer = Pubkey::new_unique();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let other_program = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![system_id.clone()])
            .with_require_one_of_programs(vec![system_id, other_program.to_string()])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    fn resolved_from_message(message: VersionedMessage) -> VersionedTransactionResolved {
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap()
    }

    #[test]
    fn test_validate_priority_fee_unset_allows_any() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let transaction = resolved_from_message(create_v1_message(
            &fee_payer,
            v1::TransactionConfig::empty().with_priority_fee(u64::MAX),
        ));

        assert!(validator.validate_priority_fee(&transaction).is_ok());
    }

    #[test]
    fn test_validate_priority_fee_enforces_cap_for_v1_config() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(10_000).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let under_cap = resolved_from_message(create_v1_message(
            &fee_payer,
            v1::TransactionConfig::empty().with_priority_fee(10_000),
        ));
        assert!(validator.validate_priority_fee(&under_cap).is_ok());

        let over_cap = resolved_from_message(create_v1_message(
            &fee_payer,
            v1::TransactionConfig::empty().with_priority_fee(10_001),
        ));
        let error = validator.validate_priority_fee(&over_cap).unwrap_err();
        assert!(
            error.to_string().contains("Priority fee 10001 exceeds maximum allowed 10000"),
            "Unexpected error: {error}"
        );
    }

    #[test]
    fn test_validate_priority_fee_enforces_cap_for_compute_budget_instructions() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(10_000).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        // 300_000 CU * 25_000 micro-lamports = 7_500 lamports, under the cap
        let under_cap = resolved_from_message(create_legacy_message(
            &fee_payer,
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(300_000),
                ComputeBudgetInstruction::set_compute_unit_price(25_000),
                transfer(&fee_payer, &recipient, 1_000),
            ],
        ));
        assert!(validator.validate_priority_fee(&under_cap).is_ok());

        // 300_000 CU * 50_000 micro-lamports = 15_000 lamports, over the cap
        let over_cap = resolved_from_message(create_legacy_message(
            &fee_payer,
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(300_000),
                ComputeBudgetInstruction::set_compute_unit_price(50_000),
                transfer(&fee_payer, &recipient, 1_000),
            ],
        ));
        let error = validator.validate_priority_fee(&over_cap).unwrap_err();
        assert!(
            error.to_string().contains("Priority fee 15000 exceeds maximum allowed 10000"),
            "Unexpected error: {error}"
        );
    }

    /// The runtime ignores ComputeBudget instructions in a V1 transaction, so the cap must
    /// read the transaction config alone. Billing the instructions instead would reject a
    /// transaction whose real priority fee is under the cap.
    #[test]
    fn test_validate_priority_fee_v1_ignores_over_cap_compute_budget_instructions() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(10_000).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        // Config asks 9_000, under the cap; the inert instructions ask
        // 300_000 CU * 100_000 micro-lamports = 30_000 lamports, well over it.
        let transaction = resolved_from_message(create_v1_message_with_instructions(
            &fee_payer,
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(300_000),
                ComputeBudgetInstruction::set_compute_unit_price(100_000),
                transfer(&fee_payer, &recipient, 1_000),
            ],
            v1::TransactionConfig::empty().with_priority_fee(9_000),
        ));

        assert!(validator.validate_priority_fee(&transaction).is_ok());
    }

    /// The mirror case: under-cap ComputeBudget instructions must not rescue a V1
    /// transaction whose config priority fee is over the cap.
    #[test]
    fn test_validate_priority_fee_v1_rejects_over_cap_config_despite_cheap_instructions() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(10_000).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        // Config asks 11_000, over the cap; the inert instructions ask
        // 300_000 CU * 10_000 micro-lamports = 3_000 lamports, under it.
        let transaction = resolved_from_message(create_v1_message_with_instructions(
            &fee_payer,
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(300_000),
                ComputeBudgetInstruction::set_compute_unit_price(10_000),
                transfer(&fee_payer, &recipient, 1_000),
            ],
            v1::TransactionConfig::empty().with_priority_fee(11_000),
        ));

        let error = validator.validate_priority_fee(&transaction).unwrap_err();
        assert!(
            error.to_string().contains("Priority fee 11000 exceeds maximum allowed 10000"),
            "Unexpected error: {error}"
        );
    }

    #[test]
    fn test_validate_priority_fee_zero_cap_blocks_priority_fees() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(0).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let no_priority_fee = resolved_from_message(create_legacy_message(
            &fee_payer,
            &[transfer(&fee_payer, &recipient, 1_000)],
        ));
        assert!(validator.validate_priority_fee(&no_priority_fee).is_ok());

        let v1_no_priority_fee =
            resolved_from_message(create_v1_message(&fee_payer, v1::TransactionConfig::empty()));
        assert!(validator.validate_priority_fee(&v1_no_priority_fee).is_ok());

        let v1_with_priority_fee = resolved_from_message(create_v1_message(
            &fee_payer,
            v1::TransactionConfig::empty().with_priority_fee(1),
        ));
        assert!(validator.validate_priority_fee(&v1_with_priority_fee).is_err());
    }

    #[test]
    fn test_validate_priority_fee_counts_alt_loaded_compute_budget() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new().with_max_priority_fee_lamports(10_000).build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let message = create_v0_message_with_alt_loaded_program(
            &fee_payer,
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(300_000).data,
                ComputeBudgetInstruction::set_compute_unit_price(50_000).data,
            ],
        );
        let resolved = create_resolved_with_loaded_keys(
            message,
            vec![fee_payer, solana_compute_budget_interface::id()],
        );

        // 300_000 CU * 50_000 micro-lamports = 15_000 lamports, over the cap;
        // the validator must see it through the resolved keys, not the static ones.
        let error = validator.validate_priority_fee(&resolved).unwrap_err();
        assert!(
            error.to_string().contains("Priority fee 15000 exceeds maximum allowed 10000"),
            "Unexpected error: {error}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_signatures() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_max_signatures(2)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instructions = vec![
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        transaction.transaction.signatures = vec![Default::default(); 3];
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_sign_and_send_transaction_mode() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], None));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_transaction() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_reject_compute_budget_only_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![solana_compute_budget_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[compute_budget_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("only ComputeBudget instructions"));
    }

    #[tokio::test]
    #[serial]
    async fn test_allow_transaction_with_compute_budget_and_non_compute_instruction() {
        let fee_payer = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                SYSTEM_PROGRAM_ID.to_string(),
                solana_compute_budget_interface::id().to_string(),
            ])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let transfer_ix = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(
            &[compute_budget_ix, transfer_ix],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_accounts() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_disallowed_accounts(vec![
                "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()
            ])
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_set_authority_new_authority() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &token_account,
            Some(&disallowed_account),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_token2022_set_authority_new_authority() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &token_account,
            Some(&disallowed_account),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_nonce_authorize_new_authority() {
        use solana_system_interface::instruction::authorize_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &disallowed_account);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_nonce_initialize_nonce_authority() {
        use solana_system_interface::instruction::create_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instructions =
            create_nonce_account(&fee_payer, &nonce_account, &disallowed_account, 1_000_000);
        // InitializeNonceAccount is the second instruction.
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_initialize_account2_owner() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::initialize_account2(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &disallowed_account,
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_initialize_mint2_freeze_authority() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::initialize_mint2(
            &spl_token_interface::id(),
            &mint,
            &fee_payer,
            Some(&disallowed_account),
            6,
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_sol_transfers() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_assign() {
        let fee_payer = Pubkey::new_unique();
        // Owner must be in the allowed programs list; the test config allows the System program.
        let new_owner = SYSTEM_PROGRAM_ID;

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_assign_rejects_off_policy_owner() {
        use solana_system_interface::instruction::assign_with_seed;

        let fee_payer = Pubkey::new_unique();
        let off_policy_owner = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = true;

        // allow_assign is true, but an owner outside the allowed programs list is still rejected.
        setup_config_with_policy(policy.clone());
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let instruction = assign_with_seed(&fee_payer, &fee_payer, "seed", &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // An owner in disallowed_accounts is rejected even when present in allowed_programs.
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string(), off_policy_owner.to_string()],
            vec![off_policy_owner.to_string()],
        );
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let instruction = assign_with_seed(&fee_payer, &fee_payer, "seed", &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_assign_owner_checked_when_reassigned_account_is_not_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let other_account = Pubkey::new_unique(); // reassigned account, not Kora
        let off_policy_owner = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();

        // allow_assign is true and the fee payer is not the reassigned account, but the owner
        // is outside the allowlist, so Kora must still refuse to sponsor the assignment.
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = true;
        setup_config_with_policy(policy.clone());
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&other_account, &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Same for an owner in disallowed_accounts, even when it is allowlisted.
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string(), off_policy_owner.to_string()],
            vec![off_policy_owner.to_string()],
        );
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&other_account, &off_policy_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let source_token_account =
            TokenAccountMockBuilder::new().with_mint(&mint).with_owner(&fee_payer).build();
        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build();

        // Plain Transfer; mint resolved from source account.
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &other_signer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_transfer_wrapped_in_batch_is_enforced() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let source_token_account =
            TokenAccountMockBuilder::new().with_mint(&mint).with_owner(&fee_payer).build();
        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build();

        let build_batched_transfer = || {
            let transfer_ix = spl_token_interface::instruction::transfer(
                &spl_token_interface::id(),
                &fee_payer_token_account,
                &recipient_token_account,
                &fee_payer,
                &[],
                1000,
            )
            .unwrap();
            let batch_ix =
                spl_token_interface::instruction::batch(&spl_token_interface::id(), &[transfer_ix])
                    .unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[batch_ix], Some(&fee_payer)));
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap()
        };

        // Batch-wrapped transfer by the fee payer must be rejected when transfers are disallowed;
        // otherwise a batch is a trivial bypass of the fee-payer policy.
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = false;
        setup_spl_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_batched_transfer();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Same batch is allowed when the policy permits fee-payer transfers.
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = true;
        setup_spl_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_batched_transfer();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_unwrap_lamports_is_enforced() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let source_token_account =
            TokenAccountMockBuilder::new().with_mint(&mint).with_owner(&fee_payer).build();
        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build();

        let build_unwrap = || {
            let ix = spl_token_interface::instruction::unwrap_lamports(
                &spl_token_interface::id(),
                &token_account,
                &destination,
                &fee_payer,
                &[],
                Some(1000),
            )
            .unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap()
        };

        // Fee payer as the unwrap authority must be rejected when the policy disallows it.
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_unwrap_lamports = false;
        setup_spl_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Allowed when the policy permits it.
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_unwrap_lamports = true;
        setup_spl_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer,
            &[],
            1,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &other_signer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_freeze_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_freeze = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_freeze = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, other_authority);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_create_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_create = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let (create_ix, _table_address) =
            alt_instruction::create_lookup_table(authority, fee_payer, 42);
        let message = VersionedMessage::Legacy(Message::new(&[create_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_create = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let (create_ix, _table_address) =
            alt_instruction::create_lookup_table(authority, fee_payer, 42);
        let message = VersionedMessage::Legacy(Message::new(&[create_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_extend_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            fee_payer,
            None,
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(fee_payer),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(fee_payer),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(Pubkey::new_unique()),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_deactivate_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_deactivate = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_deactivate = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, other_authority);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_close_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(500_000)
            .build();

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let close_ix =
            alt_instruction::close_lookup_table(lookup_table, other_authority, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transfer_amounts_rejects_alt_close_outflow_above_max_allowed_lamports() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(2_000_000)
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(matches!(
            result,
            Err(KoraError::InvalidTransaction(message))
                if message.contains("Total transfer amount 2000000 exceeds maximum allowed 1000000")
        ));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transfer_amounts_allows_alt_close_to_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(2_000_000)
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_close_account_rent_above_max_allowed_lamports_rejected() {
        let fee_payer = Pubkey::new_unique();
        let closed_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let rent_account = AccountMockBuilder::new().with_lamports(2_000_000).build();

        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_close_account = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let rpc_client = RpcMockBuilder::new().with_account_info(&rent_account).build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_2022_interface::instruction::close_account(
            &spl_token_2022_interface::id(),
            &closed_account,
            &recipient,
            &fee_payer,
            &[],
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(matches!(
            result,
            Err(KoraError::InvalidTransaction(message))
                if message.contains("Total transfer amount 2000000 exceeds maximum allowed 1000000")
        ));
    }

    #[tokio::test]
    #[serial]
    async fn test_calculate_total_outflow() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow"); // 0 - 50_000 = 0 (saturating_sub)

        let new_account = Pubkey::new_unique();
        let create_instruction =
            create_account(&fee_payer, &new_account, 200_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        let create_with_seed_instruction = create_account_with_seed(
            &fee_payer,
            &new_account,
            &fee_payer,
            "test_seed",
            300_000,
            100,
            &SYSTEM_PROGRAM_ID,
        );
        let message = VersionedMessage::Legacy(Message::new(
            &[create_with_seed_instruction],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );

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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100_000
            transfer(&sender, &fee_payer, 30_000),     // -30_000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50_000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );

        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");

        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");

        // Self-withdraw from a fee-payer-controlled nonce account is neutral.
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let nonce_account = Pubkey::new_unique();
        let withdraw_instruction = solana_system_interface::instruction::withdraw_nonce_account(
            &nonce_account,
            &fee_payer,
            &fee_payer,
            25_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 0,
            "WithdrawNonceAccount from a fee-payer-controlled nonce account back to fee payer should be neutral"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let burn_checked_ix = spl_token_interface::instruction::burn_checked(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_checked_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let closed_account = AccountMockBuilder::new().with_lamports(5_000).build();
        let rpc_client = RpcMockBuilder::new().with_account_info(&closed_account).build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_interface::instruction::close_account(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_interface::instruction::close_account(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_interface::instruction::approve(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_interface::instruction::approve(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token_interface::instruction::approve_checked(
            &spl_token_interface::id(),
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_burn = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_2022_interface::instruction::burn(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_close_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_2022_interface::instruction::close_account(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token_2022_interface::instruction::approve_checked(
            &spl_token_2022_interface::id(),
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        // Use System Program as owner since it's in allowed_programs
        let owner = SYSTEM_PROGRAM_ID;

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_allow_prefund() {
        let fee_payer = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let allowed_owner = SYSTEM_PROGRAM_ID;
        let disallowed_owner = Pubkey::new_unique();

        let build_ix =
            |new_account: Pubkey, funder: Pubkey, lamports: u64, owner: Pubkey| -> Instruction {
                let mut accounts = vec![AccountMeta::new(new_account, true)];
                if lamports > 0 {
                    accounts.push(AccountMeta::new(funder, true));
                }
                Instruction {
                    program_id: SYSTEM_PROGRAM_ID,
                    accounts,
                    data: bincode::serialize(&(DISCRIMINATOR, lamports, 100u64, owner)).unwrap(),
                }
            };

        // (label, instruction, allow_create_account, expect_ok)
        let cases = [
            // Fee payer is the funder — caught by the reused create-account gate.
            (
                "funder vector, disallowed",
                build_ix(other, fee_payer, 1000, allowed_owner),
                false,
                false,
            ),
            ("funder vector, allowed", build_ix(other, fee_payer, 1000, allowed_owner), true, true),
            // Fee payer is the prefunded account being created — the brick vector.
            (
                "brick vector, disallowed",
                build_ix(fee_payer, other, 1000, allowed_owner),
                false,
                false,
            ),
            ("brick vector, allowed", build_ix(fee_payer, other, 1000, allowed_owner), true, true),
            // lamports == 0 omits the funder; fee payer as new account must still be gated.
            (
                "brick vector no funding, disallowed",
                build_ix(fee_payer, fee_payer, 0, allowed_owner),
                false,
                false,
            ),
            // Even when allowed, the brick path must still enforce the owner allowlist.
            (
                "brick vector, owner not allowlisted",
                build_ix(fee_payer, other, 1000, disallowed_owner),
                true,
                false,
            ),
        ];

        for (label, instruction, allow, expect_ok) in cases {
            let rpc_client = RpcMockBuilder::new().build();
            let mut policy = FeePayerPolicy::default();
            policy.system.allow_create_account = allow;
            setup_config_with_policy(policy);

            let config = get_config().unwrap();
            let validator = TransactionValidator::new(config, fee_payer).unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
            let mut transaction =
                TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
            let result =
                validator.validate_transaction(config, &mut transaction, &rpc_client).await;
            assert_eq!(result.is_ok(), expect_ok, "case failed: {}", label);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_allow_prefund_via_cpi() {
        // A CreateAccountAllowPrefund surfaced as a CPI inner instruction (appended to
        // all_instructions, absent from the outer message) must still hit the policy gate.
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Outer message: a transfer not involving the fee payer — policy-neutral, keeps the tx valid.
        let outer = transfer(&new_account, &Pubkey::new_unique(), 1);
        let message = VersionedMessage::Legacy(Message::new(&[outer], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Fee payer funds the prefund create as a CPI inner instruction.
        transaction.all_instructions.push(Instruction {
            program_id: SYSTEM_PROGRAM_ID,
            accounts: vec![AccountMeta::new(new_account, true), AccountMeta::new(fee_payer, true)],
            data: bincode::serialize(&(DISCRIMINATOR, 1_000u64, 0u64, SYSTEM_PROGRAM_ID)).unwrap(),
        });

        let err = validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .expect_err("CPI prefund with fee payer as funder must be rejected");
        assert!(
            err.to_string().contains("Fee payer cannot be used for 'System Create Account'"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_rejects_disallowed_owner() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let disallowed_owner = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &disallowed_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed programs list"));
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_allows_valid_owner() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let allowed_owner = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string(), allowed_owner.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &allowed_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_create_account_with_seed_owner_checked_when_kora_is_not_create_payer() {
        use solana_system_interface::instruction::create_account_with_seed;

        let fee_payer = Pubkey::new_unique(); // Kora, the sponsor / signer
        let attacker = Pubkey::new_unique(); // the CreateAccountWithSeed `from`/payer, not Kora
        let base = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let off_policy_owner = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Kora sponsors a CreateAccountWithSeed funded by the attacker. The owner must still be
        // checked even though Kora is not the create payer.
        let instruction = create_account_with_seed(
            &attacker,
            &new_account,
            &base,
            "seed",
            1000,
            100,
            &off_policy_owner,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed programs list"));
    }

    #[tokio::test]
    #[serial]
    async fn test_create_account_with_seed_base_signer_gated_by_allow_create_account() {
        use solana_system_interface::instruction::create_account_with_seed;

        let fee_payer = Pubkey::new_unique(); // Kora, used only as the seeded base signer
        let attacker = Pubkey::new_unique(); // the create payer / funder
        let seed = "seed";
        let new_account = Pubkey::create_with_seed(&fee_payer, seed, &SYSTEM_PROGRAM_ID).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Owner is System (allowed), so the rejection must come from the allow_create_account gate
        // recognizing Kora as the seeded base signer, not from the owner allowlist.
        let instruction = create_account_with_seed(
            &attacker,
            &new_account,
            &fee_payer,
            seed,
            1000,
            0,
            &SYSTEM_PROGRAM_ID,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Fee payer cannot be used for 'System Create Account'"));
    }

    #[tokio::test]
    #[serial]
    async fn test_create_account_with_seed_base_read_from_instruction_data() {
        let fee_payer = Pubkey::new_unique(); // Kora, the seeded base in the instruction data
        let attacker = Pubkey::new_unique(); // the create payer / funder
        let decoy = Pubkey::new_unique();
        let seed = "seed";
        let new_account = Pubkey::create_with_seed(&fee_payer, seed, &SYSTEM_PROGRAM_ID).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // The authoritative base lives in the instruction data. A decoy sits at the account slot
        // a positional gate would read, with Kora referenced as a signer elsewhere — the runtime
        // accepts this (base is a signer at any index), so the gate must key off the data base.
        let instruction = Instruction::new_with_bincode(
            SYSTEM_PROGRAM_ID,
            &solana_system_interface::instruction::SystemInstruction::CreateAccountWithSeed {
                base: fee_payer,
                seed: seed.to_string(),
                lamports: 1000,
                space: 0,
                owner: SYSTEM_PROGRAM_ID,
            },
            vec![
                AccountMeta::new(attacker, true),
                AccountMeta::new(new_account, false),
                AccountMeta::new_readonly(decoy, false),
                AccountMeta::new_readonly(fee_payer, true),
            ],
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Fee payer cannot be used for 'System Create Account'"));
    }

    #[tokio::test]
    #[serial]
    async fn test_create_account_with_seed_base_equals_from_two_accounts_allowed() {
        let fee_payer = Pubkey::new_unique(); // Kora, does not participate
        let from = Pubkey::new_unique();
        let seed = "seed";
        let new_account = Pubkey::create_with_seed(&from, seed, &SYSTEM_PROGRAM_ID).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // base == from, so the runtime accepts two accounts (no separate base meta). Kora is
        // neither payer, base, nor the created account, so validation must pass.
        let instruction = Instruction::new_with_bincode(
            SYSTEM_PROGRAM_ID,
            &solana_system_interface::instruction::SystemInstruction::CreateAccountWithSeed {
                base: from,
                seed: seed.to_string(),
                lamports: 1000,
                space: 0,
                owner: SYSTEM_PROGRAM_ID,
            },
            vec![AccountMeta::new(from, true), AccountMeta::new(new_account, false)],
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_ata_create_idempotent() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        let rpc_client = RpcMockBuilder::new()
            .with_custom_mock(
                solana_client::rpc_request::RpcRequest::GetMinimumBalanceForRentExemption,
                serde_json::json!(2_039_280),
            )
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ata_program_id.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let ata_ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &fee_payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );
        let message = VersionedMessage::Legacy(Message::new(&[ata_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;

        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("ATA creation"));
            }
            _ => panic!("Expected ATA create policy violation"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_ata_create_idempotent_charged_in_outflow_without_inner_create() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        let rpc_client = RpcMockBuilder::new()
            .with_custom_mock(
                solana_client::rpc_request::RpcRequest::GetMinimumBalanceForRentExemption,
                serde_json::json!(2_039_280),
            )
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ata_program_id.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let ata_ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &fee_payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );
        let message = VersionedMessage::Legacy(Message::new(&[ata_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;

        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("Total transfer amount"));
                assert!(msg.contains("exceeds maximum allowed"));
            }
            _ => panic!("Expected outflow limit violation for ATA creation"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_allocate() {
        use solana_system_interface::instruction::allocate;

        let fee_payer = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_initialize() {
        use solana_system_interface::instruction::create_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        // Only test the InitializeNonceAccount instruction (second one)
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_advance() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_advance = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_advance = false;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_withdraw() {
        use solana_system_interface::instruction::withdraw_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_nonce_self_withdraw_does_not_hide_excess_fee_payer_outflow() {
        use solana_system_interface::instruction::withdraw_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = true;
        policy.system.nonce.allow_withdraw = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(800)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = vec![
            withdraw_nonce_account(&nonce_account, &fee_payer, &fee_payer, 1_000),
            transfer(&fee_payer, &recipient, 900),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("Total transfer amount"));
                assert!(msg.contains("exceeds maximum allowed"));
            }
            _ => panic!("Expected self-withdraw not to mask oversized fee-payer transfer"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_authorize() {
        use solana_system_interface::instruction::authorize_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_total_exceeds_fixed() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Fixed price = 5000, but total = 3000 + 2000 + 5000 = 10000 > 5000
        let fee_calc = TotalFeeCalculation::new(5000, 3000, 2000, 5000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_err());
        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("Strict pricing violation"));
            assert!(msg.contains("exceeds fixed price"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    #[serial]
    fn test_strict_pricing_total_within_fixed() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Fixed price = 5000, total = 1000 + 1000 + 1000 = 3000 < 5000
        let fee_calc = TotalFeeCalculation::new(5000, 1000, 1000, 1000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_disabled() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: false,
        };
        let _ = update_config(config);

        let fee_calc = TotalFeeCalculation::new(5000, 10000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok(), "Should pass when strict=false");
    }

    #[test]
    #[serial]
    fn test_strict_pricing_with_margin_pricing() {
        use crate::{
            fee::price::PriceModel, state::update_config, tests::config_mock::ConfigMockBuilder,
        };

        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Margin { margin: 0.1 };
        let _ = update_config(config);

        let fee_calc = TotalFeeCalculation::new(5000, 10000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_exact_match() {
        use crate::{
            fee::price::PriceModel, state::update_config, tests::config_mock::ConfigMockBuilder,
        };

        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Total exactly equals fixed price (5000 = 5000)
        let fee_calc = TotalFeeCalculation::new(5000, 2000, 1000, 2000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok(), "Should pass when total equals fixed price");
    }

    #[test]
    #[serial]
    fn test_strict_pricing_sub_lamport_quote_rejected() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 1,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Sub-lamport configured price floors to 0 but real cost is positive (base_fee = 5000).
        let fee_calc = TotalFeeCalculation::new(0, 5000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_err(), "Strict mode must reject a zero quote with positive real cost");
        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("Strict pricing violation"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transaction_rejected_by_default() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique(); // Different from fee payer

        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Durable transactions"));
            assert!(msg.contains("not allowed"));
        } else {
            panic!("Expected InvalidTransaction error");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transaction_allowed_when_enabled() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique(); // Different from fee payer

        let mock_config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .with_allow_durable_transactions(true)
            .build();
        update_config(mock_config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_non_durable_transaction_passes() {
        let fee_payer = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_revoke() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for revoke policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_revoke() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_revoke = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_2022_interface::instruction::revoke(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_revoke = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_2022_interface::instruction::revoke(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_revoke policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_set_authority() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for set_authority policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_set_authority() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_set_authority policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_mint_to() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_mint_to = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_interface::instruction::mint_to(
            &spl_token_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_mint_to = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_interface::instruction::mint_to(
            &spl_token_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for mint_to policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_mint_to() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_mint_to = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_2022_interface::instruction::mint_to(
            &spl_token_2022_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_mint_to = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_2022_interface::instruction::mint_to(
            &spl_token_2022_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_mint_to policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_mint() {
        let fee_payer = Pubkey::new_unique();
        let mint_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // fee_payer is the mint_authority (encoded in instruction data)
        let init_mint_ix = spl_token_interface::instruction::initialize_mint(
            &spl_token_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_interface::instruction::initialize_mint(
            &spl_token_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_mint policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_mint() {
        let fee_payer = Pubkey::new_unique();
        let mint_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_mint = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_2022_interface::instruction::initialize_mint(
            &spl_token_2022_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_mint = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_2022_interface::instruction::initialize_mint(
            &spl_token_2022_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_mint policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // initialize_account puts owner at account index 2 (token_account, mint, owner, rent_sysvar)
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_interface::instruction::initialize_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_interface::instruction::initialize_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_2022_interface::instruction::initialize_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_2022_interface::instruction::initialize_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_multisig() {
        let fee_payer = Pubkey::new_unique();
        let multisig_account = Pubkey::new_unique();
        let other_signer = Pubkey::new_unique();

        // fee_payer is one of the signers (parsed from accounts[2..])
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_multisig = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_interface::instruction::initialize_multisig(
            &spl_token_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_multisig = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_interface::instruction::initialize_multisig(
            &spl_token_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_multisig policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_multisig() {
        let fee_payer = Pubkey::new_unique();
        let multisig_account = Pubkey::new_unique();
        let other_signer = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_multisig = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_2022_interface::instruction::initialize_multisig(
            &spl_token_2022_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_multisig = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_2022_interface::instruction::initialize_multisig(
            &spl_token_2022_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_multisig policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_freeze_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // freeze_account(program_id, account, mint, freeze_authority, signers) — freeze_authority at index 2
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_freeze_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_interface::instruction::freeze_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_freeze_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_interface::instruction::freeze_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for freeze_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_freeze_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_freeze_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_2022_interface::instruction::freeze_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_freeze_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_2022_interface::instruction::freeze_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_freeze_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_thaw_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // thaw_account(program_id, account, mint, freeze_authority, signers) — freeze_authority at index 2
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_thaw_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_interface::instruction::thaw_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_thaw_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_interface::instruction::thaw_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for thaw_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_thaw_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_2022_interface::instruction::thaw_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_2022_interface::instruction::thaw_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_thaw_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_withdraw_excess_lamports_is_enforced() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let build_withdraw = || {
            let ix = spl_token_2022_interface::instruction::withdraw_excess_lamports(
                &spl_token_2022_interface::id(),
                &token_account,
                &destination,
                &fee_payer,
                &[],
            )
            .unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap()
        };

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_withdraw_excess_lamports = false;
        setup_token2022_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_withdraw();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("WithdrawExcessLamports"));
        } else {
            panic!("Expected InvalidTransaction error for token2022 WithdrawExcessLamports policy");
        }

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_withdraw_excess_lamports = true;
        setup_token2022_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_withdraw();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_unwrap_lamports_is_enforced() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let build_unwrap = || {
            let ix = spl_token_2022_interface::instruction::unwrap_lamports(
                &spl_token_2022_interface::id(),
                &token_account,
                &destination,
                &fee_payer,
                &[],
                Some(1000),
            )
            .unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap()
        };

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_unwrap_lamports = false;
        policy.spl_token.allow_unwrap_lamports = true;
        setup_token2022_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("UnwrapLamports"));
        } else {
            panic!("Expected InvalidTransaction error for token2022 UnwrapLamports policy");
        }

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_unwrap_lamports = true;
        setup_token2022_config_with_policy(policy);
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let mut transaction = build_unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_reallocate_rejected_for_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let reallocate_ix = spl_token_2022_interface::instruction::reallocate(
            &spl_token_2022_interface::id(),
            &token_account,
            &fee_payer,
            &fee_payer,
            &[],
            &[spl_token_2022_interface::extension::ExtensionType::MemoTransfer],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[reallocate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Token2022 Reallocate is not allowed"));
        } else {
            panic!("Expected InvalidTransaction error for token2022 reallocate");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_pause_with_fee_payer_rejected() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::pause(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("Token2022 Pause")),
            "Expected rejection when fee payer is the pausable authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_resume_allowed_when_policy_explicitly_enabled() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::resume(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_ok(), "Explicit thaw opt-in should allow Token2022 Resume: {result:?}");
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_pausable_initialize_rejects_fee_payer_authority_by_default() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("Token2022 InitializePausable authority")),
            "Expected rejection when fee payer is assigned as pausable authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_update_rejected_when_policy_denies() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let mut config = get_config().unwrap().clone();
        config.validation.token_2022.transfer_hook_policy = TransferHookPolicy::DenyAll;
        config.validation.fee_payer_policy.token_2022.allow_update_extension_authority = true;
        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        validator.validate_transaction(&config, &mut transaction, &rpc_client).await.unwrap();

        let result = validator.validate_token2022_transfer_hook_signing_policies(
            &config,
            &mut transaction,
            TransferHookValidationFlow::DelayedSigning,
        );
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("TransferHook")),
            "Expected transfer-hook policy rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_update_allowed_for_immediate_send_when_policy_is_delayed_only(
    ) {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let mut config = get_config().unwrap().clone();
        config.validation.token_2022.transfer_hook_policy =
            TransferHookPolicy::DenyMutableForDelayedSigning;
        config.validation.fee_payer_policy.token_2022.allow_update_extension_authority = true;
        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        validator.validate_transaction(&config, &mut transaction, &rpc_client).await.unwrap();

        let result = validator.validate_token2022_transfer_hook_signing_policies(
            &config,
            &mut transaction,
            TransferHookValidationFlow::ImmediateSignAndSend,
        );
        assert!(
            result.is_ok(),
            "Immediate-sign flow should be allowed under delayed-only policy: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_initialize_disallowed_program_id_rejected() {
        let fee_payer = Keypair::new();
        let disallowed_program_id = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_program_id.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            Some(authority),
            Some(disallowed_program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferHook program_id")),
            "Expected disallowed transfer-hook program id rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_cpi_rejects_fee_payer_as_extra_account() {
        use crate::transaction::IxUtils;
        use solana_message::AccountKeys;
        use solana_transaction_status::parse_instruction;
        use solana_transaction_status_client_types::{UiInstruction, UiParsedInstruction};

        let fee_payer = Pubkey::new_unique();
        let reconstruct_hooked_transfer = |extra_account: Pubkey| {
            let mut hooked = spl_token_2022_interface::instruction::transfer_checked(
                &spl_token_2022_interface::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
                1,
                2,
            )
            .unwrap();
            hooked.accounts.extend([
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(extra_account, false),
            ]);
            let message = Message::new(std::slice::from_ref(&hooked), None);
            let parsed = parse_instruction::parse(
                &spl_token_2022_interface::id(),
                &message.instructions[0],
                &AccountKeys::new(&message.account_keys, None),
                None,
            )
            .unwrap();
            let mut account_keys = message.account_keys.clone();
            let compiled = IxUtils::reconstruct_instruction_from_ui(
                &UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed)),
                &mut account_keys,
            )
            .unwrap();
            IxUtils::uncompile_instructions(&[compiled], &account_keys).unwrap().remove(0)
        };

        for (extra_account, expect_denied) in [(fee_payer, true), (Pubkey::new_unique(), false)] {
            let rpc_client = RpcMockBuilder::new().with_mint_account(2).build();
            let mut policy = FeePayerPolicy::default();
            policy.token_2022.allow_transfer = false;
            setup_token2022_config_with_policy(policy);
            let config = get_config().unwrap();
            let validator = TransactionValidator::new(config, fee_payer).unwrap();

            let outer = spl_token_2022_interface::instruction::sync_native(
                &spl_token_2022_interface::id(),
                &Pubkey::new_unique(),
            )
            .unwrap();
            let message = VersionedMessage::Legacy(Message::new(&[outer], Some(&fee_payer)));
            let mut transaction =
                TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
            transaction.all_instructions.push(reconstruct_hooked_transfer(extra_account));

            let result =
                validator.validate_transaction(config, &mut transaction, &rpc_client).await;
            if expect_denied {
                assert!(
                    matches!(result, Err(KoraError::InvalidTransaction(ref msg))
                        if msg.contains("Fee payer cannot be used for 'Token2022 Token Transfer'")),
                    "fee payer as a hook extra account must be denied, got {result:?}"
                );
            } else {
                assert!(result.is_ok(), "hooked transfer without fee payer failed: {result:?}");
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_transfer_fee_config_rejects_fee_payer_authorities_by_default(
    ) {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&fee_payer.pubkey()),
            Some(&Pubkey::new_unique()),
            25,
            100,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferFeeConfig transferFeeConfigAuthority")),
            "Expected rejection when fee payer is planted as transfer-fee authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_transfer_fee_config_disallowed_withdraw_authority_rejected()
    {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let disallowed_authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_authority.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&Pubkey::new_unique()),
            Some(&disallowed_authority),
            25,
            100,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferFeeConfig withdrawWithheldAuthority")),
            "Expected disallowed withdraw authority rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_update_metadata_pointer_rejects_fee_payer_authority_by_default() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("current Token2022 extension authority")),
            "Expected rejection when fee payer updates metadata pointer, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_update_metadata_pointer_allowed_when_extension_update_policy_enabled() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_update_extension_authority = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            result.is_ok(),
            "Explicit extension update opt-in should allow metadata pointer updates: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_mint_close_authority_rejects_disallowed_new_authority() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let disallowed_authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_authority.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::instruction::initialize_mint_close_authority(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&disallowed_authority),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeMintCloseAuthority newAuthority")),
            "Expected disallowed mint close authority rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_metadata_pointer_rejects_blocked_extension() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_programs =
            ProgramsConfig::Allowlist(vec![spl_token_2022_interface::id().to_string()]);
        config.validation.token_2022.blocked_mint_extensions = vec!["metadata_pointer".to_string()];
        config.validation.token_2022.initialize().unwrap();
        let _config_guard = setup_config_mock(config.clone());

        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            Some(Pubkey::new_unique()),
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(&config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("extension 'MetadataPointer' is blocked")),
            "Expected blocked metadata pointer extension rejection, got: {result:?}"
        );
    }

    async fn assert_blocked_token2022_extension_rejected(
        instruction: solana_sdk::instruction::Instruction,
        block_account_extensions: Vec<String>,
        block_mint_extensions: Vec<String>,
        expected_extension: &str,
    ) {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_programs =
            ProgramsConfig::Allowlist(vec![spl_token_2022_interface::id().to_string()]);
        config.validation.token_2022.blocked_account_extensions = block_account_extensions;
        config.validation.token_2022.blocked_mint_extensions = block_mint_extensions;
        config.validation.token_2022.allow_token_metadata_instructions = true;
        config.validation.token_2022.allow_token_group_instructions = true;
        config.validation.token_2022.initialize().unwrap();
        setup_both_configs(config.clone());

        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();

        // Fee payer is deliberately not one of the instruction accounts, so the rejection must
        // come from the blocked-extension check rather than the fee-payer-presence check.
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(&config, &mut transaction, &rpc_client).await;
        let expected = format!("extension '{expected_extension}' is blocked");
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains(&expected)),
            "Expected blocked {expected_extension} rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_memo_transfer_rejects_blocked_account_extension() {
        let instruction =
            spl_token_2022_interface::extension::memo_transfer::instruction::enable_required_transfer_memos(
                &spl_token_2022_interface::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
            )
            .unwrap();
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec!["memo_transfer".to_string()],
            vec![],
            "MemoTransfer",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_cpi_guard_rejects_blocked_account_extension() {
        let instruction =
            spl_token_2022_interface::extension::cpi_guard::instruction::enable_cpi_guard(
                &spl_token_2022_interface::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
            )
            .unwrap();
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec!["cpi_guard".to_string()],
            vec![],
            "CpiGuard",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_immutable_owner_rejects_blocked_account_extension() {
        let instruction = spl_token_2022_interface::instruction::initialize_immutable_owner(
            &spl_token_2022_interface::id(),
            &Pubkey::new_unique(),
        )
        .unwrap();
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec!["immutable_owner".to_string()],
            vec![],
            "ImmutableOwner",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_non_transferable_rejects_blocked_mint_extension() {
        let instruction = spl_token_2022_interface::instruction::initialize_non_transferable_mint(
            &spl_token_2022_interface::id(),
            &Pubkey::new_unique(),
        )
        .unwrap();
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec![],
            vec!["non_transferable".to_string()],
            "NonTransferable",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_default_account_state_rejects_blocked_mint_extension() {
        let instruction =
            spl_token_2022_interface::extension::default_account_state::instruction::initialize_default_account_state(
                &spl_token_2022_interface::id(),
                &Pubkey::new_unique(),
                &spl_token_2022_interface::state::AccountState::Frozen,
            )
            .unwrap();
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec!["default_account_state".to_string()],
            vec![],
            "DefaultAccountState",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_confidential_extension_instructions_rejected() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let confidential_ix = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![],
            data: spl_token_2022_interface::instruction::TokenInstruction::ConfidentialTransferExtension
                .pack(),
        };

        let message = VersionedMessage::Legacy(Message::new(&[confidential_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Confidential Token-2022 instructions are not supported"));
        } else {
            panic!("Expected InvalidTransaction error for confidential token2022 instruction");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_confidential_mint_burn_rejected_by_default() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let confidential_ix = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![],
            data: spl_token_2022_interface::instruction::TokenInstruction::ConfidentialMintBurnExtension
                .pack(),
        };

        let message = VersionedMessage::Legacy(Message::new(&[confidential_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Confidential Token-2022 instructions are not supported"));
        } else {
            panic!("Expected InvalidTransaction error for confidential mint/burn instruction");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_confidential_extensions_allowed_when_flag_enabled() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_confidential_allowed(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let confidential_variants = [
            spl_token_2022_interface::instruction::TokenInstruction::ConfidentialTransferExtension,
            spl_token_2022_interface::instruction::TokenInstruction::ConfidentialTransferFeeExtension,
            spl_token_2022_interface::instruction::TokenInstruction::ConfidentialMintBurnExtension,
        ];

        for variant in confidential_variants {
            let confidential_ix = Instruction {
                program_id: spl_token_2022_interface::id(),
                accounts: vec![AccountMeta::new(Pubkey::new_unique(), false)],
                data: variant.pack(),
            };

            let message =
                VersionedMessage::Legacy(Message::new(&[confidential_ix], Some(&fee_payer)));
            let mut transaction =
                TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

            assert!(
                validator.validate_transaction(config, &mut transaction, &rpc_client).await.is_ok(),
                "Confidential {variant:?} should pass when allow_confidential_transfers is enabled"
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_confidential_extension_rejects_fee_payer_account() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_confidential_allowed(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let confidential_ix = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![AccountMeta::new(fee_payer, false)],
            data: spl_token_2022_interface::instruction::TokenInstruction::ConfidentialMintBurnExtension
                .pack(),
        };

        let message = VersionedMessage::Legacy(Message::new(&[confidential_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(
                msg.contains("Fee payer cannot be an account in"),
                "Expected fee payer rejection, got: {msg}"
            );
        } else {
            panic!(
                "Expected InvalidTransaction error when fee payer is in confidential instruction"
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_mixed_instructions() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            500,
        )
        .unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(
            validator.validate_transaction(config, &mut transaction, &rpc_client).await.is_ok(),
            "Both policies true should pass"
        );

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        policy.spl_token.allow_burn = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for burn policy");
        }

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = false;
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for revoke policy");
        }
    }

    fn setup_loader_v4_config_with_policy(policy: FeePayerPolicy) {
        use crate::constant::LOADER_V4_PROGRAM_ID;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                LOADER_V4_PROGRAM_ID.to_string(),
                SYSTEM_PROGRAM_ID.to_string(),
            ])
            .with_max_allowed_lamports(10_000_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_write_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_write = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::write(&program, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::write(&program, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_deploy_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_deploy = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::deploy(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::deploy(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_copy_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let source = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_copy = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::copy(&destination, &fee_payer, &source, 0, 0, 64);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::copy(&destination, &fee_payer, &source, 0, 0, 64);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_retract_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_retract = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::retract(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_accepts_self_recipient() {
        // When the fee payer is authority AND recipient, freed lamports return to Kora.
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_set_program_length = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 1024, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_rejects_foreign_recipient() {
        // Drainage guard: when Kora is authority, any recipient other than Kora is rejected.
        // Shrinking to zero with a user recipient would drain Kora's rent lamports.
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let user_recipient = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_set_program_length = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 0, &user_recipient);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let err = validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .expect_err("shrink to foreign recipient must be rejected");
        if let KoraError::InvalidTransaction(msg) = err {
            assert!(msg.contains("drainage guard"));
        } else {
            panic!("expected InvalidTransaction, got {err:?}");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_rejects_when_policy_disabled() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 1024, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_transfer_authority_denied_by_default() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::transfer_authority(&program, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_transfer_authority_rejects_when_fee_payer_is_new_authority() {
        // Regression: loader-v4 TransferAuthority requires BOTH current and new authority to
        // sign. The fee payer's transaction-level signature satisfies the new_authority slot,
        // so an attacker can set current_authority=attacker and new_authority=fee_payer and
        // silently push program authority onto Kora. The validator must reject this even when
        // current_authority != fee_payer.
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let attacker = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // current_authority = attacker (not fee_payer), new_authority = fee_payer.
        let ix = loader_v4::transfer_authority(&program, &attacker, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_finalize_denied_by_default() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let next_version = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::finalize(&program, &fee_payer, &next_version);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    fn setup_bpf_v3_config_with_policy(policy: FeePayerPolicy) {
        use crate::constant::BPF_LOADER_UPGRADEABLE_PROGRAM_ID;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string(),
                SYSTEM_PROGRAM_ID.to_string(),
            ])
            .with_max_allowed_lamports(10_000_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_write_requires_policy() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.bpf_loader_upgradeable.allow_write = true;
        setup_bpf_v3_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::write(&buffer, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let ix = loader_v3::write(&buffer, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_set_authority_denied_by_default() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::set_buffer_authority(&buffer, &fee_payer, &user);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_close_with_foreign_recipient_blocked_by_drainage_guard() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let attacker = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.bpf_loader_upgradeable.allow_close = true;
        setup_bpf_v3_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::close(&buffer, &attacker, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let err = validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .expect_err("foreign recipient must be rejected");
        if let KoraError::InvalidTransaction(msg) = err {
            assert!(msg.contains("drainage guard"), "got: {msg}");
        } else {
            panic!("expected InvalidTransaction, got {err:?}");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_close_with_self_recipient_accepted() {
        // Closing back to Kora is fine — Kora reclaims its own rent.
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.bpf_loader_upgradeable.allow_close = true;
        setup_bpf_v3_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::close(&buffer, &fee_payer, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_upgrade_with_kora_as_spill_does_not_require_allow_upgrade() {
        // Regression: spill is a lamport recipient, not a signer. A user upgrading their own
        // program who picks Kora as the spill account is just refunding excess lamports;
        // it should not trigger allow_upgrade gating.
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let user_authority = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // user_authority signs as upgrade_authority; spill = fee_payer.
        let ix = loader_v3::upgrade(&program, &buffer, &user_authority, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_close_with_kora_as_recipient_does_not_require_allow_close() {
        // Regression: recipient is a lamport sink, not a signer. A user closing their own
        // buffer with Kora as recipient is just sending us money — should not be gated.
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let user_authority = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::close(&buffer, &fee_payer, &user_authority);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_extend_program_checked_with_kora_payer_denied_by_default() {
        // Regression: ExtendProgramChecked previously fell through a `_ => {}` wildcard in
        // the parser and was completely unreached by the policy. An attacker with their own
        // upgradeable program could submit one that named Kora as the optional payer at
        // index 4 — Kora would silently fund the extension. Now `allow_extend_program_checked`
        // gates it (default false).
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let attacker = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let programdata = Pubkey::new_unique();
        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // attacker is the authority; fee_payer is the (optional) payer.
        let ix = loader_v3::extend_program_checked(&program, &attacker, Some(&fee_payer), 128);
        // Force payer to be the fee_payer by re-deriving programdata.
        let _ = programdata; // silence warning
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_bpf_v3_migrate_denied_by_default() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let programdata = Pubkey::new_unique();
        setup_bpf_v3_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v3::migrate_program(&programdata, &program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_instructions_rejected_by_default() {
        let fee_payer = Pubkey::new_unique();
        let metadata = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_metadata_interface::instruction::remove_key(
            &spl_token_2022_interface::id(),
            &metadata,
            &authority,
            "some-key".to_string(),
            false,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("token-metadata interface instructions are not supported")),
            "Expected default-off rejection of metadata interface instructions, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_group_instructions_rejected_by_default() {
        let fee_payer = Pubkey::new_unique();
        let group = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_group_interface::instruction::update_group_max_size(
            &spl_token_2022_interface::id(),
            &group,
            &authority,
            32,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("token-group interface instructions are not supported")),
            "Expected default-off rejection of group interface instructions, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_instructions_rejected_when_token_metadata_blocked() {
        let instruction = spl_token_metadata_interface::instruction::remove_key(
            &spl_token_2022_interface::id(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            "some-key".to_string(),
            false,
        );
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec![],
            vec!["token_metadata".to_string()],
            "TokenMetadata",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_group_instructions_rejected_when_token_group_blocked() {
        let instruction = spl_token_group_interface::instruction::update_group_max_size(
            &spl_token_2022_interface::id(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            32,
        );
        assert_blocked_token2022_extension_rejected(
            instruction,
            vec![],
            vec!["token_group".to_string()],
            "TokenGroup",
        )
        .await;
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_remove_key_rejects_fee_payer_as_current_authority() {
        let fee_payer = Pubkey::new_unique();
        let metadata = Pubkey::new_unique();
        setup_token2022_config_interface_allowed(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Attacker tries to borrow the fee payer's signature as the metadata update
        // authority (fee payer occupies the authority account slot).
        let instruction = spl_token_metadata_interface::instruction::remove_key(
            &spl_token_2022_interface::id(),
            &metadata,
            &fee_payer,
            "some-key".to_string(),
            false,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("current Token2022 extension authority")),
            "Expected rejection via the extension-authority policy check, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_remove_key_allowed_when_update_extension_authority_policy_enabled() {
        let fee_payer = Pubkey::new_unique();
        let metadata = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_update_extension_authority = true;
        setup_token2022_config_interface_allowed(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Same instruction as the rejection test above, but the operator has
        // explicitly opted in to the fee payer acting as an extension authority.
        let instruction = spl_token_metadata_interface::instruction::remove_key(
            &spl_token_2022_interface::id(),
            &metadata,
            &fee_payer,
            "some-key".to_string(),
            false,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            result.is_ok(),
            "allow_update_extension_authority opt-in should allow metadata authority use: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_initialize_allowed_when_fee_payer_absent() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let update_authority = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        setup_token2022_config_interface_allowed(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // A legitimate token issuance: the fee payer only pays; it is not an
        // account in the metadata instruction. Previously this was rejected at
        // parse time (0xc); it must now be accepted.
        let instruction = spl_token_metadata_interface::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            &update_authority,
            &mint,
            &mint_authority,
            "USDP".to_string(),
            "USDP".to_string(),
            "https://example.com/usdp.json".to_string(),
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_update_authority_rejects_planting_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let metadata = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        setup_token2022_config_interface_allowed(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Fee payer is NOT an account here; it is planted as the new authority
        // via instruction data. Must be caught by the planted-authority check.
        let instruction = spl_token_metadata_interface::instruction::update_authority(
            &spl_token_2022_interface::id(),
            &metadata,
            &current_authority,
            MaybeNull::try_from(Some(fee_payer)).unwrap(),
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("planted as a Token2022 extension authority")),
            "Expected rejection via the planted-authority policy check, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_metadata_update_authority_allows_planted_fee_payer_when_policy_enabled() {
        let fee_payer = Pubkey::new_unique();
        let metadata = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_extension_authority = true;
        setup_token2022_config_interface_allowed(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_metadata_interface::instruction::update_authority(
            &spl_token_2022_interface::id(),
            &metadata,
            &current_authority,
            MaybeNull::try_from(Some(fee_payer)).unwrap(),
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            result.is_ok(),
            "allow_initialize_extension_authority opt-in should allow assigning the fee payer: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_group_update_authority_rejects_planting_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let group = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        setup_token2022_config_interface_allowed(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_group_interface::instruction::update_group_authority(
            &spl_token_2022_interface::id(),
            &group,
            &current_authority,
            Some(fee_payer),
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("planted as a Token2022 extension authority")),
            "Expected rejection via the planted-authority policy check, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_group_update_max_size_allowed_when_update_extension_authority_policy_enabled() {
        let fee_payer = Pubkey::new_unique();
        let group = Pubkey::new_unique();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_update_extension_authority = true;
        setup_token2022_config_interface_allowed(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_group_interface::instruction::update_group_max_size(
            &spl_token_2022_interface::id(),
            &group,
            &fee_payer,
            32,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            result.is_ok(),
            "allow_update_extension_authority opt-in should allow group authority use: {result:?}"
        );
    }
}

// Fuzzes the fee-payer policy matrix: for each gated program role, asserts the validator
// gates the fee payer exactly when the role's flag is off, and never gates a non-fee-payer.
#[cfg(test)]
mod fee_payer_policy_props;
