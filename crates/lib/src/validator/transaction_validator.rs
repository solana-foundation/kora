use crate::{
    config::{Config, FeePayerPolicy},
    error::KoraError,
    fee::fee::{FeeConfigUtil, TotalFeeCalculation},
    oracle::PriceSource,
    token::{interface::TokenMint, token::TokenUtil},
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, ParsedSystemInstructionData,
        ParsedSystemInstructionType, VersionedTransactionResolved,
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::str::FromStr;

use crate::fee::price::PriceModel;

pub struct TransactionValidator {
    fee_payer_pubkey: Pubkey,
    max_allowed_lamports: u64,
    allowed_programs: Vec<Pubkey>,
    max_signatures: u64,
    allowed_tokens: Vec<Pubkey>,
    disallowed_accounts: Vec<Pubkey>,
    _price_source: PriceSource,
    fee_payer_policy: FeePayerPolicy,
    allow_durable_transactions: bool,
}

impl TransactionValidator {
    pub fn new(config: &Config, fee_payer_pubkey: Pubkey) -> Result<Self, KoraError> {
        let config = &config.validation;

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
                .map(|addr| Pubkey::from_str(addr))
                .collect::<Result<Vec<Pubkey>, _>>()
                .map_err(|e| {
                    KoraError::InternalServerError(format!("Invalid allowed token address: {e}"))
                })?,
            disallowed_accounts: config
                .disallowed_accounts
                .iter()
                .map(|addr| Pubkey::from_str(addr))
                .collect::<Result<Vec<Pubkey>, _>>()
                .map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid disallowed account address: {e}"
                    ))
                })?,
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
        // First check if the mint is in allowed tokens
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

        if transaction_resolved.all_account_keys.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no account keys".to_string(),
            ));
        }

        self.validate_signatures(&transaction_resolved.transaction)?;

        self.validate_programs(transaction_resolved)?;
        self.validate_transfer_amounts(config, transaction_resolved, rpc_client).await?;
        self.validate_disallowed_accounts(transaction_resolved)?;
        self.validate_fee_payer_usage(transaction_resolved)?;

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

    fn validate_fee_payer_usage(
        &self,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        let system_instructions = transaction_resolved.get_or_parse_system_instructions()?;

        // Check for durable transactions (nonce-based) - reject if not allowed
        if !self.allow_durable_transactions
            && system_instructions
                .contains_key(&ParsedSystemInstructionType::SystemAdvanceNonceAccount)
        {
            return Err(KoraError::InvalidTransaction(
                "Durable transactions (nonce-based) are not allowed".to_string(),
            ));
        }

        // Validate system program instructions
        validate_system!(self, system_instructions, SystemTransfer,
            ParsedSystemInstructionData::SystemTransfer { sender, .. } => sender,
            self.fee_payer_policy.system.allow_transfer, "System Transfer");

        validate_system!(self, system_instructions, SystemAssign,
            ParsedSystemInstructionData::SystemAssign { authority } => authority,
            self.fee_payer_policy.system.allow_assign, "System Assign");

        validate_system!(self, system_instructions, SystemAllocate,
            ParsedSystemInstructionData::SystemAllocate { account } => account,
            self.fee_payer_policy.system.allow_allocate, "System Allocate");

        validate_system!(self, system_instructions, SystemCreateAccount,
            ParsedSystemInstructionData::SystemCreateAccount { payer, .. } => payer,
            self.fee_payer_policy.system.allow_create_account, "System Create Account");

        validate_system!(self, system_instructions, SystemInitializeNonceAccount,
            ParsedSystemInstructionData::SystemInitializeNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_initialize, "System Initialize Nonce Account");

        validate_system!(self, system_instructions, SystemAdvanceNonceAccount,
            ParsedSystemInstructionData::SystemAdvanceNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_advance, "System Advance Nonce Account");

        validate_system!(self, system_instructions, SystemAuthorizeNonceAccount,
            ParsedSystemInstructionData::SystemAuthorizeNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_authorize, "System Authorize Nonce Account");

        // Note: SystemUpgradeNonceAccount not validated - no authority parameter

        validate_system!(self, system_instructions, SystemWithdrawNonceAccount,
            ParsedSystemInstructionData::SystemWithdrawNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_withdraw, "System Withdraw Nonce Account");

        // Validate SPL instructions
        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;

        validate_spl!(self, spl_instructions, SplTokenTransfer,
            ParsedSPLInstructionData::SplTokenTransfer { owner, is_2022, .. } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_transfer,
            self.fee_payer_policy.token_2022.allow_transfer,
            "SPL Token Transfer", "Token2022 Token Transfer");

        validate_spl!(self, spl_instructions, SplTokenApprove,
            ParsedSPLInstructionData::SplTokenApprove { owner, is_2022, .. } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_approve,
            self.fee_payer_policy.token_2022.allow_approve,
            "SPL Token Approve", "Token2022 Token Approve");

        validate_spl!(self, spl_instructions, SplTokenBurn,
            ParsedSPLInstructionData::SplTokenBurn { owner, is_2022 } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_burn,
            self.fee_payer_policy.token_2022.allow_burn,
            "SPL Token Burn", "Token2022 Token Burn");

        validate_spl!(self, spl_instructions, SplTokenCloseAccount,
            ParsedSPLInstructionData::SplTokenCloseAccount { owner, is_2022 } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_close_account,
            self.fee_payer_policy.token_2022.allow_close_account,
            "SPL Token Close Account", "Token2022 Token Close Account");

        validate_spl!(self, spl_instructions, SplTokenRevoke,
            ParsedSPLInstructionData::SplTokenRevoke { owner, is_2022 } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_revoke,
            self.fee_payer_policy.token_2022.allow_revoke,
            "SPL Token Revoke", "Token2022 Token Revoke");

        validate_spl!(self, spl_instructions, SplTokenSetAuthority,
            ParsedSPLInstructionData::SplTokenSetAuthority { authority, is_2022 } => { authority, is_2022 },
            self.fee_payer_policy.spl_token.allow_set_authority,
            self.fee_payer_policy.token_2022.allow_set_authority,
            "SPL Token SetAuthority", "Token2022 Token SetAuthority");

        validate_spl!(self, spl_instructions, SplTokenMintTo,
            ParsedSPLInstructionData::SplTokenMintTo { mint_authority, is_2022 } => { mint_authority, is_2022 },
            self.fee_payer_policy.spl_token.allow_mint_to,
            self.fee_payer_policy.token_2022.allow_mint_to,
            "SPL Token MintTo", "Token2022 Token MintTo");

        validate_spl!(self, spl_instructions, SplTokenInitializeMint,
            ParsedSPLInstructionData::SplTokenInitializeMint { mint_authority, is_2022 } => { mint_authority, is_2022 },
            self.fee_payer_policy.spl_token.allow_initialize_mint,
            self.fee_payer_policy.token_2022.allow_initialize_mint,
            "SPL Token InitializeMint", "Token2022 Token InitializeMint");

        validate_spl!(self, spl_instructions, SplTokenInitializeAccount,
            ParsedSPLInstructionData::SplTokenInitializeAccount { owner, is_2022 } => { owner, is_2022 },
            self.fee_payer_policy.spl_token.allow_initialize_account,
            self.fee_payer_policy.token_2022.allow_initialize_account,
            "SPL Token InitializeAccount", "Token2022 Token InitializeAccount");

        validate_spl_multisig!(self, spl_instructions, SplTokenInitializeMultisig,
            ParsedSPLInstructionData::SplTokenInitializeMultisig { signers, is_2022 } => { signers, is_2022 },
            self.fee_payer_policy.spl_token.allow_initialize_multisig,
            self.fee_payer_policy.token_2022.allow_initialize_multisig,
            "SPL Token InitializeMultisig", "Token2022 Token InitializeMultisig");

        validate_spl!(self, spl_instructions, SplTokenFreezeAccount,
            ParsedSPLInstructionData::SplTokenFreezeAccount { freeze_authority, is_2022 } => { freeze_authority, is_2022 },
            self.fee_payer_policy.spl_token.allow_freeze_account,
            self.fee_payer_policy.token_2022.allow_freeze_account,
            "SPL Token FreezeAccount", "Token2022 Token FreezeAccount");

        validate_spl!(self, spl_instructions, SplTokenThawAccount,
            ParsedSPLInstructionData::SplTokenThawAccount { freeze_authority, is_2022 } => { freeze_authority, is_2022 },
            self.fee_payer_policy.spl_token.allow_thaw_account,
            self.fee_payer_policy.token_2022.allow_thaw_account,
            "SPL Token ThawAccount", "Token2022 Token ThawAccount");

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
        transaction_resolved: &VersionedTransactionResolved,
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
        FeeConfigUtil::calculate_fee_payer_outflow(
            &self.fee_payer_pubkey,
            transaction_resolved,
            rpc_client,
            config,
        )
        .await
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
        config::FeePayerPolicy,
        state::{get_config, update_config},
        tests::{config_mock::ConfigMockBuilder, rpc_mock::RpcMockBuilder},
        transaction::TransactionUtil,
    };
    use serial_test::serial;

    use super::*;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::instruction::Instruction;
    use solana_system_interface::{
        instruction::{
            assign, create_account, create_account_with_seed, transfer, transfer_with_seed,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };

    // Helper functions to reduce test duplication and setup config
    fn setup_default_config() {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();
    }

    fn setup_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();
    }

    fn setup_spl_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();
    }

    fn setup_token2022_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let instruction = transfer(&sender, &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test transaction with amount over limit
        let instruction = transfer(&sender, &recipient, 2_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test multiple transfers
        let instructions =
            vec![transfer(&sender, &recipient, 500_000), transfer(&sender, &recipient, 500_000)];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test allowed program (system program)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test disallowed program
        let fake_program = Pubkey::new_unique();
        // Create a no-op instruction for the fake program
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test too many signatures
        let instructions = vec![
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        transaction.transaction.signatures = vec![Default::default(); 3]; // Add 3 dummy signatures
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test SignAndSend mode with fee payer already set should not error
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test SignAndSend mode without fee payer (should succeed)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], None)); // No fee payer specified
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        // Create an empty message using Message::new with empty instructions
        let message = VersionedMessage::Legacy(Message::new(&[], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_sol_transfers() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test with allow_sol_transfers = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_sol_transfers = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_assign() {
        let fee_payer = Pubkey::new_unique();
        let new_owner = Pubkey::new_unique();

        // Test with allow_assign = true

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_assign = false

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();

        // Test with allow_spl_transfers = true
        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_spl_transfers = false
        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with other account as source - should always pass
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &other_signer, // other account is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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

        // Test with allow_token2022_transfers = true
        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        // Test with token_2022.allow_transfer = true
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_token2022_transfers = false
        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer is not allowed to be source
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with other account as source - should always pass
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass because fee payer is not the source
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        // Test 1: Fee payer as sender in Transfer - should add to outflow
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
            .await
            .unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient in Transfer - should subtract from outflow (account closure)
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
            .await
            .unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");

        // Test 8: Other account funding CreateAccount - should not affect outflow
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow = validator
            .calculate_total_outflow(&config, &mut transaction, &rpc_client)
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

        // Test with allow_burn = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should pass because allow_burn is true by default
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_burn = false

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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

        // Should fail because fee payer cannot burn tokens when allow_burn is false
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test burn_checked instruction
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

        // Should also fail for burn_checked
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should pass because allow_close_account is true by default
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_close_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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

        // Should fail because fee payer cannot close accounts when allow_close_account is false
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should pass because allow_approve is true by default
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_approve = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test approve_checked instruction
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

        // Should also fail for approve_checked
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_burn = false for Token2022

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_burn = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should fail for Token2022 burn
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = false for Token2022

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_close_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should fail for Token2022 close account
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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
        // Should pass because allow_approve is true by default
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_approve = false

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

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

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test approve_checked instruction
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

        // Should also fail for approve_checked
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        // Test with allow_create_account = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_create_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_allocate() {
        use solana_system_interface::instruction::allocate;

        let fee_payer = Pubkey::new_unique();

        // Test with allow_allocate = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_allocate = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_initialize() {
        use solana_system_interface::instruction::create_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        // Test with allow_initialize = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        // Only test the InitializeNonceAccount instruction (second one)
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_advance() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        // Test with allow_advance = true (must also enable durable transactions)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_advance = false (durable txs enabled but policy blocks it)
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
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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

        // Test with allow_withdraw = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_withdraw = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_authorize() {
        use solana_system_interface::instruction::authorize_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        // Test with allow_authorize = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_authorize = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(&config, &mut transaction, &rpc_client)
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
        let result = TransactionValidator::validate_strict_pricing_with_fee(&config, &fee_calc);

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
        let result = TransactionValidator::validate_strict_pricing_with_fee(&config, &fee_calc);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_disabled() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: false, // Disabled
        };
        let _ = update_config(config);

        let fee_calc = TotalFeeCalculation::new(5000, 10000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(&config, &fee_calc);

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
        let result = TransactionValidator::validate_strict_pricing_with_fee(&config, &fee_calc);

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
        let result = TransactionValidator::validate_strict_pricing_with_fee(&config, &fee_calc);

        assert!(result.is_ok(), "Should pass when total equals fixed price");
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transaction_rejected_by_default() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique(); // Different from fee payer

        // Default config has allow_durable_transactions = false
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(&mut transaction, &rpc_client).await;
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

        // Enable durable transactions
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let validator = TransactionValidator::new(fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass because durable transactions are allowed
        assert!(validator.validate_transaction(&mut transaction, &rpc_client).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_non_durable_transaction_passes() {
        let fee_payer = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Default config has allow_durable_transactions = false
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let validator = TransactionValidator::new(fee_payer).unwrap();

        // Regular transfer (no nonce instruction)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass - no AdvanceNonceAccount instruction
        assert!(validator.validate_transaction(&mut transaction, &rpc_client).await.is_ok());
    }
}
