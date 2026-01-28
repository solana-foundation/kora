use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{compiled_instruction::CompiledInstruction, VersionedMessage};
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
    /// Add a fee payer balance assertion to a transaction if lighthouse is enabled.
    /// Asserts that fee payer balance >= (current_balance - estimated_fee) at transaction end.
    pub async fn add_fee_payer_assertion(
        transaction: &mut VersionedTransaction,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        estimated_fee: u64,
        config: &LighthouseConfig,
    ) -> Result<(), KoraError> {
        if !config.enabled {
            return Ok(());
        }

        let current_balance = rpc_client.get_balance(fee_payer).await?;
        let min_expected = current_balance.saturating_sub(estimated_fee);

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
    ) -> Result<u8, KoraError> {
        if let Some(index) = account_keys.iter().position(|k| k == pubkey) {
            Ok(index as u8)
        } else {
            if account_keys.len() >= 256 {
                return Err(KoraError::ValidationError(
                    "Transaction has too many accounts (max 256)".to_string(),
                ));
            }
            let index = account_keys.len() as u8;
            account_keys.push(*pubkey);
            Ok(index)
        }
    }

    /// Append an instruction to a versioned transaction
    fn append_instruction_to_transaction(
        transaction: &mut VersionedTransaction,
        instruction: Instruction,
    ) -> Result<(), KoraError> {
        match &mut transaction.message {
            VersionedMessage::Legacy(message) => {
                let program_id_index =
                    Self::find_or_add_account(&mut message.account_keys, &instruction.program_id)?;

                let account_indices: Vec<u8> = instruction
                    .accounts
                    .iter()
                    .map(|meta| Self::find_or_add_account(&mut message.account_keys, &meta.pubkey))
                    .collect::<Result<Vec<_>, _>>()?;

                message.instructions.push(CompiledInstruction {
                    program_id_index,
                    accounts: account_indices,
                    data: instruction.data,
                });

                Ok(())
            }
            VersionedMessage::V0(message) => {
                let program_id_index =
                    Self::find_or_add_account(&mut message.account_keys, &instruction.program_id)?;

                let account_indices: Vec<u8> = instruction
                    .accounts
                    .iter()
                    .map(|meta| Self::find_or_add_account(&mut message.account_keys, &meta.pubkey))
                    .collect::<Result<Vec<_>, _>>()?;

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

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_ok());

        assert_eq!(transaction.message.instructions().len(), original_ix_count + 1);
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

        let assertion_ix = LighthouseUtil::build_fee_payer_assertion(&keypair.pubkey(), 1_000_000);
        let config = LighthouseConfig { enabled: true, fail_if_transaction_size_overflow: true };

        let result =
            LighthouseUtil::append_lighthouse_assertion(&mut transaction, assertion_ix, &config);
        assert!(result.is_ok());

        assert_eq!(transaction.message.instructions().len(), original_ix_count + 1);
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
}
