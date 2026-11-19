use solana_client::nonblocking::rpc_client::RpcClient;
use solana_compute_budget::compute_budget_limits::{
    MAX_COMPUTE_UNIT_LIMIT, MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES,
};
use solana_message::{
    compiled_instruction::CompiledInstruction, v1, MessageHeader, VersionedMessage,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::{
    config::LighthouseConfig, constant::LIGHTHOUSE_PROGRAM_ID, error::KoraError, sanitize_error,
    transaction::TransactionUtil,
};

/// Lighthouse instruction discriminators
const ASSERT_ACCOUNT_INFO_DISCRIMINATOR: u8 = 5;

/// LogLevel::Silent value
const LOG_LEVEL_SILENT: u8 = 0;

/// IntegerOperator::GreaterThanOrEqual value (matches Lighthouse SDK)
const INTEGER_OPERATOR_GTE: u8 = 4;

/// AccountInfoAssertion::Lamports variant (index 0 in the enum)
const ACCOUNT_INFO_ASSERTION_LAMPORTS: u8 = 0;

/// Lighthouse has no upgrade authority, so what the runtime charges to load it is fixed: 64
/// bytes plus the data length for the program account (36) and for its program data (424,293).
const LIGHTHOUSE_LOADED_ACCOUNTS_DATA_SIZE: u32 = 424_457;

/// The assertion consumes 678 compute units on a fee payer, which holds no account data.
const ASSERTION_COMPUTE_UNITS: u32 = 1_000;

pub struct LighthouseUtil {}

impl LighthouseUtil {
    /// Add a fee payer balance assertion to a transaction if lighthouse is enabled and not sending.
    /// Asserts that fee payer balance >= (current_balance - estimated_fee) at transaction end.
    ///
    /// The `will_send` parameter indicates if the transaction will be sent to the network directly.
    /// When `will_send` is true, the assertion is skipped because modifying the message would
    /// invalidate existing client signatures.
    ///
    /// Returns whether the assertion was appended. `false` when lighthouse is disabled, when the
    /// transaction will be sent by Kora, or when the assertion was skipped for size.
    pub async fn add_fee_payer_assertion(
        transaction: &mut VersionedTransaction,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        estimated_fee: u64,
        config: &LighthouseConfig,
        will_send: bool,
    ) -> Result<bool, KoraError> {
        if !config.enabled || will_send {
            return Ok(false);
        }

        let current_balance = rpc_client.get_balance(fee_payer).await.map_err(|e| {
            KoraError::RpcError(format!(
                "Failed to fetch fee payer balance for Lighthouse assertion: {}",
                sanitize_error!(e)
            ))
        })?;
        let min_expected = current_balance.saturating_sub(estimated_fee);

        if min_expected == 0 {
            log::warn!(
                "Fee payer {} has balance {} which may be insufficient for estimated fee {}",
                fee_payer,
                current_balance,
                estimated_fee
            );
        }

        let assertion_ix = Self::build_fee_payer_assertion(fee_payer, min_expected);
        Self::append_lighthouse_assertion(transaction, assertion_ix, config)
    }

    fn build_assert_account_info_data(min_lamports: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(12);

        data.push(ASSERT_ACCOUNT_INFO_DISCRIMINATOR);

        data.push(LOG_LEVEL_SILENT);

        data.push(ACCOUNT_INFO_ASSERTION_LAMPORTS);

        // Lamports value (u64 little-endian)
        data.extend_from_slice(&min_lamports.to_le_bytes());

        data.push(INTEGER_OPERATOR_GTE);

        data
    }

    /// Build a Lighthouse assertion instruction that asserts the fee payer's balance
    /// is >= min_lamports at the end of the transaction.
    fn build_fee_payer_assertion(fee_payer: &Pubkey, min_lamports: u64) -> Instruction {
        let data = Self::build_assert_account_info_data(min_lamports);

        Instruction {
            program_id: LIGHTHOUSE_PROGRAM_ID,
            accounts: vec![AccountMeta::new_readonly(*fee_payer, false)],
            data,
        }
    }

    fn find_or_add_account(
        account_keys: &mut Vec<Pubkey>,
        pubkey: &Pubkey,
    ) -> Result<(u8, bool), KoraError> {
        if let Some(index) = account_keys.iter().position(|k| k == pubkey) {
            Ok((index as u8, false))
        } else {
            if account_keys.len() >= 256 {
                return Err(KoraError::ValidationError(
                    "Transaction has too many accounts (max 256)".to_string(),
                ));
            }
            let index = account_keys.len() as u8;
            account_keys.push(*pubkey);
            Ok((index, true))
        }
    }

    fn increment_readonly_unsigned_accounts(header: &mut MessageHeader) -> Result<(), KoraError> {
        header.num_readonly_unsigned_accounts =
            header.num_readonly_unsigned_accounts.checked_add(1).ok_or_else(|| {
                KoraError::ValidationError(
                    "num_readonly_unsigned_accounts overflow when appending instruction"
                        .to_string(),
                )
            })?;
        Ok(())
    }

    /// Compile an instruction against a message's account keys, appending any new
    /// accounts as readonly unsigned.
    fn compile_appended_instruction(
        account_keys: &mut Vec<Pubkey>,
        header: &mut MessageHeader,
        instruction: Instruction,
    ) -> Result<CompiledInstruction, KoraError> {
        let (program_id_index, program_added) =
            Self::find_or_add_account(account_keys, &instruction.program_id)?;
        if program_added {
            Self::increment_readonly_unsigned_accounts(header)?;
        }

        let mut account_indices: Vec<u8> = Vec::with_capacity(instruction.accounts.len());
        for meta in &instruction.accounts {
            let (index, added) = Self::find_or_add_account(account_keys, &meta.pubkey)?;
            if added {
                if meta.is_signer || meta.is_writable {
                    return Err(KoraError::ValidationError(
                        "Appending new signer/writable accounts is not supported".to_string(),
                    ));
                }
                Self::increment_readonly_unsigned_accounts(header)?;
            }
            account_indices.push(index);
        }

        Ok(CompiledInstruction {
            program_id_index,
            accounts: account_indices,
            data: instruction.data,
        })
    }

    fn append_instruction_to_transaction(
        transaction: &mut VersionedTransaction,
        instruction: Instruction,
    ) -> Result<(), KoraError> {
        let VersionedMessage::V1(message) = &mut transaction.message else {
            return Err(KoraError::ValidationError(
                "Lighthouse assertions require a V1 transaction".to_string(),
            ));
        };
        let compiled = Self::compile_appended_instruction(
            &mut message.account_keys,
            &mut message.header,
            instruction,
        )?;
        message.instructions.push(compiled);
        Ok(())
    }

    fn reserve_assertion_resources(config: &mut v1::TransactionConfig) {
        config.compute_unit_limit = config
            .compute_unit_limit
            .map(|limit| limit.saturating_add(ASSERTION_COMPUTE_UNITS).min(MAX_COMPUTE_UNIT_LIMIT));
        config.loaded_accounts_data_size_limit =
            config.loaded_accounts_data_size_limit.map(|limit| {
                limit
                    .saturating_add(LIGHTHOUSE_LOADED_ACCOUNTS_DATA_SIZE)
                    .min(MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get())
            });
    }

    /// Handles size overflow based on config settings.
    /// Returns `true` when the assertion was appended, `false` when it was skipped for size.
    pub(crate) fn append_lighthouse_assertion(
        transaction: &mut VersionedTransaction,
        assertion_ix: Instruction,
        config: &LighthouseConfig,
    ) -> Result<bool, KoraError> {
        // Clone and append to get actual size
        let mut tx_with_assertion = transaction.clone();
        Self::append_instruction_to_transaction(&mut tx_with_assertion, assertion_ix)?;
        if let VersionedMessage::V1(message) = &mut tx_with_assertion.message {
            Self::reserve_assertion_resources(&mut message.config);
        }

        let new_size = TransactionUtil::serialize_versioned_transaction(&tx_with_assertion)
            .map_err(|e| {
                KoraError::SerializationError(sanitize_error!(format!(
                    "Failed to serialize transaction: {e}"
                )))
            })?
            .len();

        // Appending the assertion adds accounts and an instruction, either of which can push
        // the message past a limit that is not about byte size
        let rejection = if let Err(e) = tx_with_assertion.message.sanitize() {
            Some(format!("would produce an invalid message ({e})"))
        } else {
            let max_size = v1::MAX_TRANSACTION_SIZE;
            (new_size > max_size)
                .then(|| format!("would exceed transaction size limit ({new_size} > {max_size})"))
        };

        if let Some(reason) = rejection {
            if config.fail_if_transaction_size_overflow {
                return Err(KoraError::ValidationError(format!(
                    "Adding Lighthouse assertion {reason}"
                )));
            } else {
                log::warn!("Lighthouse assertion {reason}. Skipping.");
                return Ok(false);
            }
        }

        *transaction = tx_with_assertion;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::transaction_mock::{
        create_mock_v1_transaction_with_max_addresses, create_test_message,
    };
    use solana_message::{v1, VersionedMessage};
    use solana_sdk::{hash::Hash, instruction::AccountMeta, signature::Keypair, signer::Signer};

    #[test]
    fn test_build_assert_account_info_data() {
        let data = LighthouseUtil::build_assert_account_info_data(1_000_000);

        // Verify structure: discriminator(1) + log_level(1) + variant(1) + u64(8) + operator(1) = 12 bytes
        assert_eq!(data.len(), 12);
        assert_eq!(data[0], 5); // ASSERT_ACCOUNT_INFO_DISCRIMINATOR
        assert_eq!(data[1], 0); // LogLevel::Silent
        assert_eq!(data[2], 0); // ACCOUNT_INFO_ASSERTION_LAMPORTS
                                // Bytes 3-10: u64 little-endian (1_000_000 = 0x000F4240)
        assert_eq!(u64::from_le_bytes(data[3..11].try_into().unwrap()), 1_000_000);
        assert_eq!(data[11], 4); // IntegerOperator::GreaterThanOrEqual
    }

    #[test]
    fn test_build_fee_payer_assertion() {
        let fee_payer = Keypair::new().pubkey();
        let min_lamports = 1_000_000;

        let ix = LighthouseUtil::build_fee_payer_assertion(&fee_payer, min_lamports);

        assert_eq!(ix.data.len(), 12);
        assert_eq!(ix.accounts.len(), 1);
        assert_eq!(ix.accounts[0].pubkey, fee_payer);
        assert!(!ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable);
    }

    #[test]
    fn test_append_lighthouse_assertion() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();

        let v1_message = v1::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            config: v1::TransactionConfig::empty(),
            lifetime_specifier: Hash::new_unique(),
            account_keys: vec![keypair.pubkey(), program_id],
            instructions: vec![solana_message::compiled_instruction::CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: vec![1, 2, 3],
            }],
        };

        let message = VersionedMessage::V1(v1_message);
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let original_ix_count = transaction.message.instructions().len();
        let original_readonly_unsigned =
            transaction.message.header().num_readonly_unsigned_accounts;

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_ok());

        assert_eq!(transaction.message.instructions().len(), original_ix_count + 1);
        assert_eq!(
            transaction.message.header().num_readonly_unsigned_accounts,
            original_readonly_unsigned + 1
        );
        assert!(transaction.message.static_account_keys().contains(&LIGHTHOUSE_PROGRAM_ID));
    }

    #[test]
    fn test_append_lighthouse_assertion_reserves_resources_for_the_assertion() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(Pubkey::new_unique(), &[1, 2, 3], vec![]);
        let build = |compute_unit_limit, loaded_accounts_data_size_limit| {
            let message = v1::Message::try_compile_with_config(
                &keypair.pubkey(),
                std::slice::from_ref(&instruction),
                Hash::new_unique(),
                v1::TransactionConfig::empty()
                    .with_compute_unit_limit(compute_unit_limit)
                    .with_loaded_accounts_data_size_limit(loaded_accounts_data_size_limit),
            )
            .unwrap();
            let mut transaction =
                VersionedTransaction::try_new(VersionedMessage::V1(message), &[&keypair]).unwrap();
            let appended = LighthouseUtil::append_lighthouse_assertion(
                &mut transaction,
                LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000),
                &LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true },
            )
            .unwrap();
            assert!(appended);
            let VersionedMessage::V1(message) = transaction.message else {
                panic!("expected a V1 message");
            };
            message.config
        };

        let config = build(5_000, 100_000);
        assert_eq!(config.compute_unit_limit, Some(5_000 + ASSERTION_COMPUTE_UNITS));
        assert_eq!(
            config.loaded_accounts_data_size_limit,
            Some(100_000 + LIGHTHOUSE_LOADED_ACCOUNTS_DATA_SIZE)
        );

        let config = build(MAX_COMPUTE_UNIT_LIMIT, MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get());
        assert_eq!(config.compute_unit_limit, Some(MAX_COMPUTE_UNIT_LIMIT));
        assert_eq!(
            config.loaded_accounts_data_size_limit,
            Some(MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get())
        );
    }

    #[test]
    fn test_append_lighthouse_assertion_rejects_exceeding_address_limit() {
        let keypair = Keypair::new_from_array([1; 32]);
        let mut transaction = create_mock_v1_transaction_with_max_addresses(&keypair);

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);

        let error = result.expect_err("assertion must not push the message past 64 addresses");
        assert!(
            error.to_string().contains("would produce an invalid message"),
            "unexpected error: {error}"
        );
        assert!(!transaction.message.static_account_keys().contains(&LIGHTHOUSE_PROGRAM_ID));
    }

    #[test]
    fn test_append_lighthouse_assertion_skips_exceeding_address_limit_when_configured() {
        let keypair = Keypair::new_from_array([2; 32]);
        let mut transaction = create_mock_v1_transaction_with_max_addresses(&keypair);
        let original_ix_count = transaction.message.instructions().len();

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: false };

        let appended =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config)
                .expect("assertion is skipped, not an error");
        assert!(!appended, "a skipped assertion must be reported as not appended");

        assert_eq!(transaction.message.instructions().len(), original_ix_count);
        assert!(!transaction.message.static_account_keys().contains(&LIGHTHOUSE_PROGRAM_ID));
    }

    #[test]
    fn test_append_lighthouse_assertion_header_unchanged_when_lighthouse_program_exists() {
        let keypair = Keypair::new();

        let instruction = Instruction::new_with_bytes(
            LIGHTHOUSE_PROGRAM_ID,
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();
        let original_readonly_unsigned =
            transaction.message.header().num_readonly_unsigned_accounts;

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_ok());
        assert_eq!(
            transaction.message.header().num_readonly_unsigned_accounts,
            original_readonly_unsigned
        );
    }

    #[test]
    fn test_overflow_skip_behavior() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();

        let large_data = vec![0u8; 3900];
        let instruction = Instruction::new_with_bytes(
            program_id,
            &large_data,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();
        let size = TransactionUtil::serialize_versioned_transaction(&transaction).unwrap().len();
        assert!(size <= v1::MAX_TRANSACTION_SIZE, "fixture must fit before the assertion: {size}");

        let original_ix_count = transaction.message.instructions().len();

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: false };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_ok());

        assert_eq!(transaction.message.instructions().len(), original_ix_count);
    }

    #[test]
    fn test_overflow_fail_behavior() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();

        let large_data = vec![0u8; 3900];
        let instruction = Instruction::new_with_bytes(
            program_id,
            &large_data,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();
        let size = TransactionUtil::serialize_versioned_transaction(&transaction).unwrap().len();
        assert!(size <= v1::MAX_TRANSACTION_SIZE, "fixture must fit before the assertion: {size}");

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_err());

        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("exceed transaction size limit"));
        } else {
            panic!("Expected ValidationError");
        }
    }
}
