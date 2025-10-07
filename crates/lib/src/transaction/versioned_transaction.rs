use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_commitment_config::CommitmentConfig;
use solana_message::{v0::MessageAddressTableLookup, VersionedMessage};
use solana_sdk::{
    instruction::{CompiledInstruction, Instruction},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::{collections::HashMap, ops::Deref};

use solana_transaction_status_client_types::UiInstruction;

use crate::{
    error::KoraError,
    fee::fee::{FeeConfigUtil, TransactionFeeUtil},
    signer::KoraSigner,
    state::get_config,
    transaction::{
        instruction_util::IxUtils, ParsedSPLInstructionData, ParsedSPLInstructionType,
        ParsedSystemInstructionData, ParsedSystemInstructionType,
    },
    validator::transaction_validator::TransactionValidator,
    CacheUtil, Signer,
};
use solana_address_lookup_table_interface::state::AddressLookupTable;

/// A fully resolved transaction with lookup tables and inner instructions resolved
pub struct VersionedTransactionResolved {
    pub transaction: VersionedTransaction,

    // Includes lookup table addresses
    pub all_account_keys: Vec<Pubkey>,

    // Includes all instructions, including inner instructions
    pub all_instructions: Vec<Instruction>,

    // Parsed instructions by type (None if not parsed yet)
    parsed_system_instructions:
        Option<HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>>,

    // Parsed SPL instructions by type (None if not parsed yet)
    parsed_spl_instructions:
        Option<HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>>,
}

impl Deref for VersionedTransactionResolved {
    type Target = VersionedTransaction;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

#[async_trait]
pub trait VersionedTransactionOps {
    fn encode_b64_transaction(&self) -> Result<String, KoraError>;
    fn find_signer_position(&self, signer_pubkey: &Pubkey) -> Result<usize, KoraError>;

    async fn sign_transaction(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError>;
    async fn sign_transaction_if_paid(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError>;
    async fn sign_and_send_transaction(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError>;
}

impl VersionedTransactionResolved {
    pub async fn from_transaction(
        transaction: &VersionedTransaction,
        rpc_client: &RpcClient,
        sig_verify: bool,
    ) -> Result<Self, KoraError> {
        let mut resolved = Self {
            transaction: transaction.clone(),
            all_account_keys: vec![],
            all_instructions: vec![],
            parsed_system_instructions: None,
            parsed_spl_instructions: None,
        };

        // 1. Resolve lookup table addresses based on transaction type
        let resolved_addresses = match &transaction.message {
            VersionedMessage::Legacy(_) => {
                // Legacy transactions don't have lookup tables
                vec![]
            }
            VersionedMessage::V0(v0_message) => {
                // V0 transactions may have lookup tables
                LookupTableUtil::resolve_lookup_table_addresses(
                    rpc_client,
                    &v0_message.address_table_lookups,
                )
                .await?
            }
        };

        // Set all accout keys
        let mut all_account_keys = transaction.message.static_account_keys().to_vec();
        all_account_keys.extend(resolved_addresses.clone());
        resolved.all_account_keys = all_account_keys.clone();

        // 2. Fetch all instructions
        let outer_instructions =
            IxUtils::uncompile_instructions(transaction.message.instructions(), &all_account_keys);

        let inner_instructions = resolved.fetch_inner_instructions(rpc_client, sig_verify).await?;

        resolved.all_instructions.extend(outer_instructions);
        resolved.all_instructions.extend(inner_instructions);

        Ok(resolved)
    }

    /// Only use this is we built the transaction ourselves, because it won't do any checks for resolving LUT, etc.
    pub fn from_kora_built_transaction(transaction: &VersionedTransaction) -> Self {
        Self {
            transaction: transaction.clone(),
            all_account_keys: transaction.message.static_account_keys().to_vec(),
            all_instructions: IxUtils::uncompile_instructions(
                transaction.message.instructions(),
                transaction.message.static_account_keys(),
            ),
            parsed_system_instructions: None,
            parsed_spl_instructions: None,
        }
    }

    /// Fetch inner instructions via simulation
    async fn fetch_inner_instructions(
        &mut self,
        rpc_client: &RpcClient,
        sig_verify: bool,
    ) -> Result<Vec<Instruction>, KoraError> {
        let simulation_result = rpc_client
            .simulate_transaction_with_config(
                &self.transaction,
                RpcSimulateTransactionConfig {
                    commitment: Some(rpc_client.commitment()),
                    sig_verify,
                    inner_instructions: true,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to simulate transaction: {e}")))?;

        if let Some(err) = simulation_result.value.err {
            let mut error_msg = format!("{}", err);
            
            // Add detailed logs if available
            if let Some(logs) = &simulation_result.value.logs {
                if !logs.is_empty() {
                    let log_details = logs.iter()
                        .map(|log| format!("  {}", log))
                        .collect::<Vec<_>>()
                        .join("\n");
                    error_msg.push_str(&format!("\nLogs:\n{}", log_details));
                }
            }
            
            return Err(KoraError::TransactionSimulationFailed(error_msg));
        }

        if let Some(inner_instructions) = simulation_result.value.inner_instructions {
            let mut compiled_inner_instructions: Vec<CompiledInstruction> = vec![];

            inner_instructions.iter().for_each(|ix| {
                ix.instructions.iter().for_each(|inner_ix| {
                    if let UiInstruction::Compiled(ix) = inner_ix {
                        compiled_inner_instructions.push(CompiledInstruction {
                            program_id_index: ix.program_id_index,
                            accounts: ix.accounts.clone(),
                            data: bs58::decode(&ix.data).into_vec().unwrap_or_default(),
                        });
                    }
                });
            });

            return Ok(IxUtils::uncompile_instructions(
                &compiled_inner_instructions,
                &self.all_account_keys,
            ));
        }

        Ok(vec![])
    }

    pub fn get_or_parse_system_instructions(
        &mut self,
    ) -> Result<&HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>, KoraError>
    {
        if self.parsed_system_instructions.is_none() {
            self.parsed_system_instructions = Some(IxUtils::parse_system_instructions(self)?);
        }
        Ok(self.parsed_system_instructions.as_ref().unwrap())
    }

    pub fn get_or_parse_spl_instructions(
        &mut self,
    ) -> Result<&HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>, KoraError> {
        if self.parsed_spl_instructions.is_none() {
            self.parsed_spl_instructions = Some(IxUtils::parse_token_instructions(self)?);
        }
        Ok(self.parsed_spl_instructions.as_ref().unwrap())
    }
}

// Implementation of the consolidated trait for VersionedTransactionResolved
#[async_trait]
impl VersionedTransactionOps for VersionedTransactionResolved {
    fn encode_b64_transaction(&self) -> Result<String, KoraError> {
        let serialized = bincode::serialize(&self.transaction).map_err(|e| {
            KoraError::SerializationError(format!("Base64 serialization failed: {e}"))
        })?;
        Ok(STANDARD.encode(serialized))
    }

    fn find_signer_position(&self, signer_pubkey: &Pubkey) -> Result<usize, KoraError> {
        self.transaction
            .message
            .static_account_keys()
            .iter()
            .position(|key| key == signer_pubkey)
            .ok_or_else(|| {
                KoraError::InvalidTransaction(format!(
                    "Signer {signer_pubkey} not found in transaction account keys"
                ))
            })
    }

    async fn sign_transaction(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError> {
        let validator = TransactionValidator::new(signer.solana_pubkey())?;

        // Validate transaction and accounts (already resolved)
        validator.validate_transaction(self).await?;

        // Get latest blockhash and update transaction
        let mut transaction = self.transaction.clone();

        if transaction.signatures.is_empty() {
            let blockhash = rpc_client
                .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
                .await?;
            transaction.message.set_recent_blockhash(blockhash.0);
        }

        // Validate transaction fee using resolved transaction
        let estimated_fee = TransactionFeeUtil::get_estimate_fee_resolved(rpc_client, self).await?;
        validator.validate_lamport_fee(estimated_fee)?;

        // Sign transaction
        let signature = signer.sign_solana(&transaction).await?;

        // Find the fee payer position - don't assume it's at position 0
        let fee_payer_position = self.find_signer_position(&signer.solana_pubkey())?;
        transaction.signatures[fee_payer_position] = signature;

        // Serialize signed transaction
        let serialized = bincode::serialize(&transaction)?;
        let encoded = STANDARD.encode(serialized);

        Ok((transaction, encoded))
    }

    async fn sign_transaction_if_paid(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError> {
        let fee_payer = signer.solana_pubkey();
        let config = &get_config()?;

        let fee_calculation = FeeConfigUtil::estimate_kora_fee(
            rpc_client,
            self,
            &fee_payer,
            config.validation.is_payment_required(),
            Some(config.validation.price_source.clone()),
        )
        .await?;

        let required_lamports = fee_calculation.total_fee_lamports;

        // Only validate payment if not free
        if required_lamports > 0 {
            // Get the expected payment destination
            let payment_destination = config.kora.get_payment_address(&signer.solana_pubkey())?;

            // Validate token payment using the resolved transaction
            TransactionValidator::validate_token_payment(
                self,
                required_lamports,
                rpc_client,
                &payment_destination,
            )
            .await?;
        }

        // Sign the transaction
        self.sign_transaction(signer, rpc_client).await
    }

    async fn sign_and_send_transaction(
        &mut self,
        signer: &std::sync::Arc<KoraSigner>,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError> {
        let (transaction, encoded) = self.sign_transaction(signer, rpc_client).await?;

        // Send and confirm transaction
        let signature = rpc_client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;

        Ok((signature.to_string(), encoded))
    }
}

pub struct LookupTableUtil {}

impl LookupTableUtil {
    /// Resolves addresses from lookup tables for V0 transactions
    pub async fn resolve_lookup_table_addresses(
        rpc_client: &RpcClient,
        lookup_table_lookups: &[MessageAddressTableLookup],
    ) -> Result<Vec<Pubkey>, KoraError> {
        let mut resolved_addresses = Vec::new();

        // Maybe we can use caching here, there's a chance the lookup tables get updated though, so tbd
        for lookup in lookup_table_lookups {
            let lookup_table_account =
                CacheUtil::get_account(rpc_client, &lookup.account_key, false).await.map_err(
                    |e| KoraError::RpcError(format!("Failed to fetch lookup table: {e}")),
                )?;

            // Parse the lookup table account data to get the actual addresses
            let address_lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data)
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to deserialize lookup table: {e}"
                    ))
                })?;

            // Resolve writable addresses
            for &index in &lookup.writable_indexes {
                if let Some(address) = address_lookup_table.addresses.get(index as usize) {
                    resolved_addresses.push(*address);
                } else {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Lookup table index {index} out of bounds for writable addresses"
                    )));
                }
            }

            // Resolve readonly addresses
            for &index in &lookup.readonly_indexes {
                if let Some(address) = address_lookup_table.addresses.get(index as usize) {
                    resolved_addresses.push(*address);
                } else {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Lookup table index {index} out of bounds for readonly addresses"
                    )));
                }
            }
        }

        Ok(resolved_addresses)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::SplTokenConfig,
        tests::{
            common::RpcMockBuilder, config_mock::mock_state::setup_config_mock,
            toml_mock::ConfigBuilder,
        },
        transaction::TransactionUtil,
        Config,
    };
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use std::collections::HashMap;

    use super::*;
    use solana_address_lookup_table_interface::state::LookupTableMeta;
    use solana_message::{v0, Message};
    use solana_sdk::{
        account::Account,
        hash::Hash,
        instruction::{AccountMeta, CompiledInstruction, Instruction},
        signature::Keypair,
        signer::Signer,
    };

    fn setup_test_config() -> Config {
        ConfigBuilder::new()
            .with_programs(vec![])
            .with_tokens(vec![])
            .with_spl_paid_tokens(SplTokenConfig::Allowlist(vec![]))
            .with_free_price()
            .with_cache_config(None, false, 60, 30) // Disable cache for tests
            .build_config()
            .expect("Failed to build test config")
    }

    #[test]
    fn test_encode_transaction_b64() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx);
        let encoded = resolved.encode_b64_transaction().unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
    }

    #[test]
    fn test_encode_decode_b64_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx);
        let encoded = resolved.encode_b64_transaction().unwrap();
        let decoded = TransactionUtil::decode_b64_transaction(&encoded).unwrap();

        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_find_signer_position_success() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        let position = transaction.find_signer_position(&keypair.pubkey()).unwrap();
        assert_eq!(position, 0); // Fee payer is typically at position 0
    }

    #[test]
    fn test_find_signer_position_success_v0() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let other_account = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![keypair.pubkey(), other_account, program_id],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2,
                accounts: vec![0, 1],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        let position = transaction.find_signer_position(&keypair.pubkey()).unwrap();
        assert_eq!(position, 0);

        let other_position = transaction.find_signer_position(&other_account).unwrap();
        assert_eq!(other_position, 1);
    }

    #[test]
    fn test_find_signer_position_middle_of_accounts() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let keypair3 = Keypair::new();
        let program_id = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 3,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![keypair1.pubkey(), keypair2.pubkey(), keypair3.pubkey(), program_id],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 3,
                accounts: vec![0, 1, 2],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        assert_eq!(transaction.find_signer_position(&keypair1.pubkey()).unwrap(), 0);
        assert_eq!(transaction.find_signer_position(&keypair2.pubkey()).unwrap(), 1);
        assert_eq!(transaction.find_signer_position(&keypair3.pubkey()).unwrap(), 2);
    }

    #[test]
    fn test_find_signer_position_not_found() {
        let keypair = Keypair::new();
        let missing_keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        let result = transaction.find_signer_position(&missing_keypair.pubkey());
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains(&missing_keypair.pubkey().to_string()));
            assert!(msg.contains("not found in transaction account keys"));
        }
    }

    #[test]
    fn test_find_signer_position_empty_account_keys() {
        // Create a transaction with minimal account keys
        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![], // Empty account keys
            recent_blockhash: Hash::default(),
            instructions: vec![],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let search_key = Pubkey::new_unique();

        let result = transaction.find_signer_position(&search_key);
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }

    #[test]
    fn test_from_kora_built_transaction() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1, 2, 3, 4],
            vec![
                AccountMeta::new(keypair.pubkey(), true),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
            ],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction.clone()], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message.clone(), &[&keypair]).unwrap();

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction);

        assert_eq!(resolved.transaction, transaction);
        assert_eq!(resolved.all_account_keys, transaction.message.static_account_keys());
        assert_eq!(resolved.all_instructions.len(), 1);

        // Check instruction properties rather than direct equality since IxUtils::uncompile_instructions
        // properly sets signer status based on the transaction message
        let resolved_instruction = &resolved.all_instructions[0];
        assert_eq!(resolved_instruction.program_id, instruction.program_id);
        assert_eq!(resolved_instruction.data, instruction.data);
        assert_eq!(resolved_instruction.accounts.len(), instruction.accounts.len());

        assert!(resolved.parsed_system_instructions.is_none());
        assert!(resolved.parsed_spl_instructions.is_none());
    }

    #[test]
    fn test_from_kora_built_transaction_v0() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let other_account = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![keypair.pubkey(), other_account, program_id],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2,
                accounts: vec![0, 1],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);
        let transaction = VersionedTransaction::try_new(message.clone(), &[&keypair]).unwrap();

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction);

        assert_eq!(resolved.transaction, transaction);
        assert_eq!(resolved.all_account_keys, vec![keypair.pubkey(), other_account, program_id]);
        assert_eq!(resolved.all_instructions.len(), 1);
        assert_eq!(resolved.all_instructions[0].program_id, program_id);
        assert_eq!(resolved.all_instructions[0].accounts.len(), 2);
        assert_eq!(resolved.all_instructions[0].data, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_from_transaction_legacy() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction.clone()], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        // Mock RPC client that will be used for inner instructions
        let mut mocks = HashMap::new();
        mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": null,
                    "logs": [],
                    "accounts": null,
                    "unitsConsumed": 1000,
                    "innerInstructions": []
                }
            }),
        );
        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let resolved =
            VersionedTransactionResolved::from_transaction(&transaction, &rpc_client, true)
                .await
                .unwrap();

        assert_eq!(resolved.transaction, transaction);
        assert_eq!(resolved.all_account_keys, transaction.message.static_account_keys());
        assert_eq!(resolved.all_instructions.len(), 1); // Only outer instruction since no inner instructions in mock

        // Check instruction properties rather than direct equality since IxUtils::uncompile_instructions
        // properly sets signer status based on the transaction message
        let resolved_instruction = &resolved.all_instructions[0];
        assert_eq!(resolved_instruction.program_id, instruction.program_id);
        assert_eq!(resolved_instruction.data, instruction.data);
        assert_eq!(resolved_instruction.accounts.len(), instruction.accounts.len());
        assert_eq!(resolved_instruction.accounts[0].pubkey, instruction.accounts[0].pubkey);
        assert_eq!(
            resolved_instruction.accounts[0].is_writable,
            instruction.accounts[0].is_writable
        );
    }

    #[tokio::test]
    async fn test_from_transaction_v0_with_lookup_tables() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let lookup_table_account = Pubkey::new_unique();
        let resolved_address = Pubkey::new_unique();

        // Create lookup table
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: u64::MAX,
                last_extended_slot: 0,
                last_extended_slot_start_index: 0,
                authority: Some(Pubkey::new_unique()),
                _padding: 0,
            },
            addresses: vec![resolved_address].into(),
        };

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![keypair.pubkey(), program_id],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0, 2], // Index 2 comes from lookup table
                data: vec![42],
            }],
            address_table_lookups: vec![solana_message::v0::MessageAddressTableLookup {
                account_key: lookup_table_account,
                writable_indexes: vec![0],
                readonly_indexes: vec![],
            }],
        };

        let message = VersionedMessage::V0(v0_message);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        // Create mock RPC client with lookup table account and simulation
        let mut mocks = HashMap::new();
        let serialized_data = lookup_table.serialize_for_tests().unwrap();
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&serialized_data);

        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": false,
                    "lamports": 0,
                    "owner": "AddressLookupTab1e1111111111111111111111111".to_string(),
                    "rentEpoch": 0
                }
            }),
        );

        mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": null,
                    "logs": [],
                    "accounts": null,
                    "unitsConsumed": 1000,
                    "innerInstructions": []
                }
            }),
        );

        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let resolved =
            VersionedTransactionResolved::from_transaction(&transaction, &rpc_client, true)
                .await
                .unwrap();

        assert_eq!(resolved.transaction, transaction);

        // Should include both static accounts and resolved addresses
        assert_eq!(resolved.all_account_keys.len(), 3); // keypair, program_id, resolved_address
        assert_eq!(resolved.all_account_keys[0], keypair.pubkey());
        assert_eq!(resolved.all_account_keys[1], program_id);
        assert_eq!(resolved.all_account_keys[2], resolved_address);
    }

    #[tokio::test]
    async fn test_from_transaction_simulation_failure() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        // Mock RPC client with simulation error
        let mut mocks = HashMap::new();
        mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": "InstructionError",
                    "logs": ["Some error log"],
                    "accounts": null,
                    "unitsConsumed": 0
                }
            }),
        );
        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let result =
            VersionedTransactionResolved::from_transaction(&transaction, &rpc_client, true).await;

        // The simulation should fail, but the exact error type depends on mock implementation
        // We expect either an RpcError (from mock deserialization) or InvalidTransaction (from simulation logic)
        assert!(result.is_err());

        match result {
            Err(KoraError::RpcError(msg)) => {
                assert!(msg.contains("Failed to simulate transaction"));
            }
            Err(KoraError::TransactionSimulationFailed(msg)) => {
                assert!(msg.contains("InstructionError"));
            }
            _ => panic!("Expected RpcError or TransactionSimulationFailed"),
        }
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_with_inner_instructions() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        // Mock RPC client with inner instructions
        let inner_instruction_data = bs58::encode(&[10, 20, 30]).into_string();
        let mut mocks = HashMap::new();
        mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": null,
                    "logs": [],
                    "accounts": null,
                    "unitsConsumed": 1000,
                    "innerInstructions": [
                        {
                            "index": 0,
                            "instructions": [
                                {
                                    "programIdIndex": 1,
                                    "accounts": [0],
                                    "data": inner_instruction_data
                                }
                            ]
                        }
                    ]
                }
            }),
        );
        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction);
        let inner_instructions =
            resolved.fetch_inner_instructions(&rpc_client, true).await.unwrap();

        assert_eq!(inner_instructions.len(), 1);
        assert_eq!(inner_instructions[0].data, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_with_sig_verify_false() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        // Mock RPC client with inner instructions
        let inner_instruction_data = bs58::encode(&[10, 20, 30]).into_string();
        let mut mocks = HashMap::new();
        mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": null,
                    "logs": [],
                    "accounts": null,
                    "unitsConsumed": 1000,
                    "innerInstructions": [
                        {
                            "index": 0,
                            "instructions": [
                                {
                                    "programIdIndex": 1,
                                    "accounts": [0],
                                    "data": inner_instruction_data
                                }
                            ]
                        }
                    ]
                }
            }),
        );
        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction);
        let inner_instructions =
            resolved.fetch_inner_instructions(&rpc_client, false).await.unwrap();

        assert_eq!(inner_instructions.len(), 1);
        assert_eq!(inner_instructions[0].data, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_get_or_parse_system_instructions() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let recipient = Pubkey::new_unique();

        // Create a system transfer instruction
        let instruction =
            solana_sdk::system_instruction::transfer(&keypair.pubkey(), &recipient, 1000000);
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction);

        // First call should parse and cache
        let parsed1_len = {
            let parsed1 = resolved.get_or_parse_system_instructions().unwrap();
            assert!(!parsed1.is_empty());
            parsed1.len()
        };

        // Second call should return cached result
        let parsed2 = resolved.get_or_parse_system_instructions().unwrap();
        assert_eq!(parsed1_len, parsed2.len());

        // Should contain transfer instruction
        assert!(
            parsed2.contains_key(&crate::transaction::ParsedSystemInstructionType::SystemTransfer)
        );
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let lookup_account_key = Pubkey::new_unique();
        let address1 = Pubkey::new_unique();
        let address2 = Pubkey::new_unique();
        let address3 = Pubkey::new_unique();

        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: u64::MAX,
                last_extended_slot: 0,
                last_extended_slot_start_index: 0,
                authority: Some(Pubkey::new_unique()),
                _padding: 0,
            },
            addresses: vec![address1, address2, address3].into(),
        };

        let serialized_data = lookup_table.serialize_for_tests().unwrap();

        let rpc_client = RpcMockBuilder::new()
            .with_account_info(&Account {
                data: serialized_data,
                executable: false,
                lamports: 0,
                owner: Pubkey::new_unique(),
                rent_epoch: 0,
            })
            .build();

        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: lookup_account_key,
            writable_indexes: vec![0, 2], // address1, address3
            readonly_indexes: vec![1],    // address2
        }];

        let resolved_addresses =
            LookupTableUtil::resolve_lookup_table_addresses(&rpc_client, &lookups).await.unwrap();

        assert_eq!(resolved_addresses.len(), 3);
        assert_eq!(resolved_addresses[0], address1);
        assert_eq!(resolved_addresses[1], address3);
        assert_eq!(resolved_addresses[2], address2);
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_empty() {
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();
        let lookups = vec![];

        let resolved_addresses =
            LookupTableUtil::resolve_lookup_table_addresses(&rpc_client, &lookups).await.unwrap();

        assert_eq!(resolved_addresses.len(), 0);
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_account_not_found() {
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();
        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: Pubkey::new_unique(),
            writable_indexes: vec![0],
            readonly_indexes: vec![],
        }];

        let result = LookupTableUtil::resolve_lookup_table_addresses(&rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::RpcError(_))));

        if let Err(KoraError::RpcError(msg)) = result {
            assert!(msg.contains("Failed to fetch lookup table"));
        }
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_invalid_index() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let lookup_account_key = Pubkey::new_unique();
        let address1 = Pubkey::new_unique();

        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: u64::MAX,
                last_extended_slot: 0,
                last_extended_slot_start_index: 0,
                authority: Some(Pubkey::new_unique()),
                _padding: 0,
            },
            addresses: vec![address1].into(), // Only 1 address, index 0
        };

        let serialized_data = lookup_table.serialize_for_tests().unwrap();
        let rpc_client = RpcMockBuilder::new()
            .with_account_info(&Account {
                data: serialized_data,
                executable: false,
                lamports: 0,
                owner: Pubkey::new_unique(),
                rent_epoch: 0,
            })
            .build();

        // Try to access index 1 which doesn't exist
        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: lookup_account_key,
            writable_indexes: vec![1], // Invalid index
            readonly_indexes: vec![],
        }];

        let result = LookupTableUtil::resolve_lookup_table_addresses(&rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("index 1 out of bounds"));
            assert!(msg.contains("writable addresses"));
        }
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_invalid_readonly_index() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let lookup_account_key = Pubkey::new_unique();
        let address1 = Pubkey::new_unique();

        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: u64::MAX,
                last_extended_slot: 0,
                last_extended_slot_start_index: 0,
                authority: Some(Pubkey::new_unique()),
                _padding: 0,
            },
            addresses: vec![address1].into(),
        };

        let serialized_data = lookup_table.serialize_for_tests().unwrap();
        let rpc_client = RpcMockBuilder::new()
            .with_account_info(&Account {
                data: serialized_data,
                executable: false,
                lamports: 0,
                owner: Pubkey::new_unique(),
                rent_epoch: 0,
            })
            .build();

        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: lookup_account_key,
            writable_indexes: vec![],
            readonly_indexes: vec![5], // Invalid index
        }];

        let result = LookupTableUtil::resolve_lookup_table_addresses(&rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("index 5 out of bounds"));
            assert!(msg.contains("readonly addresses"));
        }
    }
}
