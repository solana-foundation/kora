use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{compiled_instruction::CompiledInstruction, MessageHeader, VersionedMessage};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::{
    config::LighthouseConfig,
    constant::{LIGHTHOUSE_PROGRAM_ID, MAX_TRANSACTION_SIZE},
    error::KoraError,
    sanitize_error,
};

/// Lighthouse instruction discriminators
const ASSERT_ACCOUNT_INFO_DISCRIMINATOR: u8 = 5;

/// LogLevel::Silent value
const LOG_LEVEL_SILENT: u8 = 0;

/// IntegerOperator::GreaterThanOrEqual value (matches Lighthouse SDK)
const INTEGER_OPERATOR_GTE: u8 = 4;

/// AccountInfoAssertion::Lamports variant (index 0 in the enum)
const ACCOUNT_INFO_ASSERTION_LAMPORTS: u8 = 0;

pub struct LighthouseUtil {}

impl LighthouseUtil {
    /// Add a fee payer balance assertion to a transaction if lighthouse is enabled and not sending.
    /// Asserts that fee payer balance >= (current_balance - estimated_fee) at transaction end.
    ///
    /// The `will_send` parameter indicates if the transaction will be sent to the network directly.
    /// When `will_send` is true, the assertion is skipped because modifying the message would
    /// invalidate existing client signatures.
    pub async fn add_fee_payer_assertion(
        transaction: &mut VersionedTransaction,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        estimated_fee: u64,
        config: &LighthouseConfig,
        will_send: bool,
    ) -> Result<(), KoraError> {
        if !config.enabled || will_send {
            return Ok(());
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

    /// Build instruction data for AssertAccountInfo with Lamports assertion
    fn build_assert_account_info_data(min_lamports: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(12);

        // Instruction discriminator
        data.push(ASSERT_ACCOUNT_INFO_DISCRIMINATOR);

        // LogLevel::Silent
        data.push(LOG_LEVEL_SILENT);

        // AccountInfoAssertion::Lamports variant
        data.push(ACCOUNT_INFO_ASSERTION_LAMPORTS);

        // Lamports value (u64 little-endian)
        data.extend_from_slice(&min_lamports.to_le_bytes());

        // IntegerOperator::GreaterThanOrEqual
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

    /// Find an account in the account keys list or add it
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

    /// In a V0 message, lookup-table-loaded accounts share one index space with the static
    /// `account_keys` and are addressed at indices `>= account_keys.len()`. Inserting static keys
    /// raises that boundary, so every existing compiled-instruction index pointing into the loaded
    /// region must move up by the number of inserted keys to keep resolving the same account.
    fn shift_loaded_account_indices(
        instructions: &mut [CompiledInstruction],
        static_boundary: usize,
        inserted: usize,
    ) -> Result<(), KoraError> {
        for instruction in instructions {
            Self::shift_index_if_loaded(
                &mut instruction.program_id_index,
                static_boundary,
                inserted,
            )?;
            for index in &mut instruction.accounts {
                Self::shift_index_if_loaded(index, static_boundary, inserted)?;
            }
        }
        Ok(())
    }

    fn shift_index_if_loaded(
        index: &mut u8,
        static_boundary: usize,
        inserted: usize,
    ) -> Result<(), KoraError> {
        if (*index as usize) < static_boundary {
            return Ok(());
        }
        let shifted = (*index as usize)
            .checked_add(inserted)
            .filter(|value| *value <= u8::MAX as usize)
            .ok_or_else(|| {
                KoraError::ValidationError(
                    "Lighthouse assertion would overflow the transaction account index space"
                        .to_string(),
                )
            })?;
        *index = shifted as u8;
        Ok(())
    }

    /// Append an instruction to a versioned transaction
    fn append_instruction_to_transaction(
        transaction: &mut VersionedTransaction,
        instruction: Instruction,
    ) -> Result<(), KoraError> {
        match &mut transaction.message {
            VersionedMessage::Legacy(message) => {
                let (program_id_index, program_added) =
                    Self::find_or_add_account(&mut message.account_keys, &instruction.program_id)?;
                if program_added {
                    Self::increment_readonly_unsigned_accounts(&mut message.header)?;
                }

                let mut account_indices: Vec<u8> = Vec::with_capacity(instruction.accounts.len());
                for meta in &instruction.accounts {
                    let (index, added) =
                        Self::find_or_add_account(&mut message.account_keys, &meta.pubkey)?;
                    if added {
                        if meta.is_signer || meta.is_writable {
                            return Err(KoraError::ValidationError(
                                "Appending new signer/writable accounts is not supported"
                                    .to_string(),
                            ));
                        }
                        Self::increment_readonly_unsigned_accounts(&mut message.header)?;
                    }
                    account_indices.push(index);
                }

                message.instructions.push(CompiledInstruction {
                    program_id_index,
                    accounts: account_indices,
                    data: instruction.data,
                });

                Ok(())
            }
            VersionedMessage::V0(message) => {
                let static_keys_before = message.account_keys.len();

                let (program_id_index, program_added) =
                    Self::find_or_add_account(&mut message.account_keys, &instruction.program_id)?;
                if program_added {
                    Self::increment_readonly_unsigned_accounts(&mut message.header)?;
                }

                let mut account_indices: Vec<u8> = Vec::with_capacity(instruction.accounts.len());
                for meta in &instruction.accounts {
                    let (index, added) =
                        Self::find_or_add_account(&mut message.account_keys, &meta.pubkey)?;
                    if added {
                        if meta.is_signer || meta.is_writable {
                            return Err(KoraError::ValidationError(
                                "Appending new signer/writable accounts is not supported"
                                    .to_string(),
                            ));
                        }
                        Self::increment_readonly_unsigned_accounts(&mut message.header)?;
                    }
                    account_indices.push(index);
                }

                let inserted = message.account_keys.len() - static_keys_before;
                if inserted > 0 && !message.address_table_lookups.is_empty() {
                    Self::shift_loaded_account_indices(
                        &mut message.instructions,
                        static_keys_before,
                        inserted,
                    )?;
                }

                message.instructions.push(CompiledInstruction {
                    program_id_index,
                    accounts: account_indices,
                    data: instruction.data,
                });

                Ok(())
            }
        }
    }

    /// Append a Lighthouse assertion instruction to a transaction.
    /// Handles size overflow based on config settings.
    pub(crate) fn append_lighthouse_assertion(
        transaction: &mut VersionedTransaction,
        assertion_ix: Instruction,
        config: &LighthouseConfig,
    ) -> Result<(), KoraError> {
        // Clone and append to get actual size
        let mut tx_with_assertion = transaction.clone();
        Self::append_instruction_to_transaction(&mut tx_with_assertion, assertion_ix)?;

        let new_size = bincode::serialize(&tx_with_assertion)
            .map_err(|e| {
                KoraError::SerializationError(sanitize_error!(format!(
                    "Failed to serialize transaction: {e}"
                )))
            })?
            .len();

        if new_size > MAX_TRANSACTION_SIZE {
            if config.fail_if_transaction_size_overflow {
                return Err(KoraError::ValidationError(format!(
                    "Adding Lighthouse assertion would exceed transaction size limit ({} > {})",
                    new_size, MAX_TRANSACTION_SIZE
                )));
            } else {
                log::warn!(
                    "Lighthouse assertion would exceed transaction size limit ({} > {}). Skipping.",
                    new_size,
                    MAX_TRANSACTION_SIZE
                );
                return Ok(());
            }
        }

        // Commit the change
        *transaction = tx_with_assertion;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_message::{v0, Message, VersionedMessage};
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
    fn test_append_lighthouse_assertion_legacy() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();

        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
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
    fn test_append_lighthouse_assertion_v0() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![keypair.pubkey(), program_id],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![solana_message::compiled_instruction::CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };

        let message = VersionedMessage::V0(v0_message);
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
    fn test_append_lighthouse_assertion_header_unchanged_when_lighthouse_program_exists() {
        let keypair = Keypair::new();

        let instruction = Instruction::new_with_bytes(
            LIGHTHOUSE_PROGRAM_ID,
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
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

        let large_data = vec![0u8; 1100];
        let instruction = Instruction::new_with_bytes(
            program_id,
            &large_data,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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

        let large_data = vec![0u8; 1100];
        let instruction = Instruction::new_with_bytes(
            program_id,
            &large_data,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let mut transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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

    fn v0_transaction_with_lookup(
        account_keys: Vec<Pubkey>,
        num_readonly_unsigned_accounts: u8,
        instructions: Vec<CompiledInstruction>,
        lookup_key: Pubkey,
        writable_indexes: Vec<u8>,
    ) -> VersionedTransaction {
        let message = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts,
            },
            account_keys,
            recent_blockhash: Hash::new_unique(),
            instructions,
            address_table_lookups: vec![v0::MessageAddressTableLookup {
                account_key: lookup_key,
                writable_indexes,
                readonly_indexes: vec![],
            }],
        };

        VersionedTransaction { signatures: vec![], message: VersionedMessage::V0(message) }
    }

    #[test]
    fn test_append_v0_with_lookup_rebases_loaded_indices() {
        let fee_payer = Keypair::new().pubkey();
        let program = Pubkey::new_unique();
        let lookup_key = Pubkey::new_unique();

        let original =
            CompiledInstruction { program_id_index: 1, accounts: vec![0, 2, 3], data: vec![9] };
        let mut transaction = v0_transaction_with_lookup(
            vec![fee_payer, program],
            1,
            vec![original],
            lookup_key,
            vec![0, 1],
        );

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&fee_payer, 1_000_000);
        LighthouseUtil::append_instruction_to_transaction(&mut transaction, assertion_ix).unwrap();

        let VersionedMessage::V0(message) = &transaction.message else {
            panic!("expected V0 message");
        };

        assert_eq!(message.account_keys, vec![fee_payer, program, LIGHTHOUSE_PROGRAM_ID]);
        assert_eq!(message.header.num_readonly_unsigned_accounts, 2);

        // Loaded indices 2 and 3 must move to 3 and 4 so they still resolve to the same
        // lookup-loaded accounts now that the static boundary grew from 2 to 3.
        assert_eq!(message.instructions[0].program_id_index, 1);
        assert_eq!(message.instructions[0].accounts, vec![0, 3, 4]);

        assert_eq!(message.instructions[1].program_id_index, 2);
        assert_eq!(message.instructions[1].accounts, vec![0]);
    }

    #[test]
    fn test_append_v0_with_lookup_rebases_multiple_instructions() {
        let fee_payer = Keypair::new().pubkey();
        let program = Pubkey::new_unique();
        let lookup_key = Pubkey::new_unique();

        // Two pre-existing instructions, each mixing static (0,1) and loaded (2,3) operands, to
        // exercise the shift loop across more than one instruction.
        let ix_a =
            CompiledInstruction { program_id_index: 1, accounts: vec![0, 2, 3], data: vec![1] };
        let ix_b =
            CompiledInstruction { program_id_index: 1, accounts: vec![3, 0, 2], data: vec![2] };
        let mut transaction = v0_transaction_with_lookup(
            vec![fee_payer, program],
            1,
            vec![ix_a, ix_b],
            lookup_key,
            vec![0, 1],
        );

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&fee_payer, 1_000_000);
        LighthouseUtil::append_instruction_to_transaction(&mut transaction, assertion_ix).unwrap();

        let VersionedMessage::V0(message) = &transaction.message else {
            panic!("expected V0 message");
        };

        assert_eq!(message.account_keys, vec![fee_payer, program, LIGHTHOUSE_PROGRAM_ID]);
        // Both instructions' loaded operands (2,3) shift to (3,4); static operands (0,1) stay.
        assert_eq!(message.instructions[0].accounts, vec![0, 3, 4]);
        assert_eq!(message.instructions[1].accounts, vec![4, 0, 3]);
        // Appended assertion references the fee payer at static index 0.
        assert_eq!(message.instructions[2].program_id_index, 2);
        assert_eq!(message.instructions[2].accounts, vec![0]);
    }

    #[test]
    fn test_append_v0_with_lookup_shifts_loaded_program_id_index() {
        let fee_payer = Keypair::new().pubkey();
        let static_account = Pubkey::new_unique();
        let lookup_key = Pubkey::new_unique();

        // The invoked program is loaded from the lookup table, so program_id_index sits in the
        // loaded region and must shift along with the account operands.
        let original =
            CompiledInstruction { program_id_index: 2, accounts: vec![0, 3], data: vec![9] };
        let mut transaction = v0_transaction_with_lookup(
            vec![fee_payer, static_account],
            1,
            vec![original],
            lookup_key,
            vec![0, 1],
        );

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&fee_payer, 1_000_000);
        LighthouseUtil::append_instruction_to_transaction(&mut transaction, assertion_ix).unwrap();

        let VersionedMessage::V0(message) = &transaction.message else {
            panic!("expected V0 message");
        };

        // Lighthouse inserted at static index 2, growing the boundary from 2 to 3.
        assert_eq!(message.account_keys, vec![fee_payer, static_account, LIGHTHOUSE_PROGRAM_ID]);
        // Loaded program_id_index 2 -> 3 and loaded operand 3 -> 4.
        assert_eq!(message.instructions[0].program_id_index, 3);
        assert_eq!(message.instructions[0].accounts, vec![0, 4]);
    }

    #[test]
    fn test_append_v0_with_lookup_no_shift_when_program_present() {
        let fee_payer = Keypair::new().pubkey();
        let lookup_key = Pubkey::new_unique();

        // Lighthouse program already a static key; no new key is inserted, so nothing shifts.
        let original =
            CompiledInstruction { program_id_index: 1, accounts: vec![0, 2, 3], data: vec![9] };
        let mut transaction = v0_transaction_with_lookup(
            vec![fee_payer, LIGHTHOUSE_PROGRAM_ID],
            1,
            vec![original],
            lookup_key,
            vec![0, 1],
        );

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&fee_payer, 1_000_000);
        LighthouseUtil::append_instruction_to_transaction(&mut transaction, assertion_ix).unwrap();

        let VersionedMessage::V0(message) = &transaction.message else {
            panic!("expected V0 message");
        };

        assert_eq!(message.account_keys, vec![fee_payer, LIGHTHOUSE_PROGRAM_ID]);
        assert_eq!(message.header.num_readonly_unsigned_accounts, 1);
        assert_eq!(message.instructions[0].accounts, vec![0, 2, 3]);
        assert_eq!(message.instructions[1].accounts, vec![0]);
    }

    #[test]
    fn test_shift_index_if_loaded_bounds() {
        let mut below = 10u8;
        LighthouseUtil::shift_index_if_loaded(&mut below, 20, 5).unwrap();
        assert_eq!(below, 10);

        let mut at_boundary = 20u8;
        LighthouseUtil::shift_index_if_loaded(&mut at_boundary, 20, 3).unwrap();
        assert_eq!(at_boundary, 23);

        let mut overflow = 255u8;
        let err = LighthouseUtil::shift_index_if_loaded(&mut overflow, 0, 1).unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }
}
