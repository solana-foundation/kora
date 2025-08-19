use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_client::nonblocking::rpc_client::RpcClient;
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
    get_signer,
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
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError>;
    async fn sign_transaction_if_paid(
        &mut self,
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError>;
    async fn sign_and_send_transaction(
        &mut self,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError>;
}

impl VersionedTransactionResolved {
    pub async fn from_transaction(
        transaction: &VersionedTransaction,
        rpc_client: &RpcClient,
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

        let inner_instructions = resolved.fetch_inner_instructions(rpc_client).await?;

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
    ) -> Result<Vec<Instruction>, KoraError> {
        let simulation_result = rpc_client
            .simulate_transaction(&self.transaction)
            .await
            .map_err(|e| KoraError::RpcError(format!("Failed to simulate transaction: {e}")))?;

        if let Some(err) = simulation_result.value.err {
            log::warn!(
                "Transaction simulation failed: {err}, continuing without inner instructions",
            );
            return Err(KoraError::InvalidTransaction(
                "Transaction inner instructions fetching failed.".to_string(),
            ));
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
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError> {
        let signer = get_signer()?;
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

        // Validate transaction fee
        let estimated_fee =
            TransactionFeeUtil::get_estimate_fee(rpc_client, &transaction.message).await?;
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
        rpc_client: &RpcClient,
    ) -> Result<(VersionedTransaction, String), KoraError> {
        let signer = get_signer()?;
        let fee_payer = signer.solana_pubkey();
        let config = &get_config()?;

        // Get the simulation result for fee calculation
        let min_transaction_fee = FeeConfigUtil::estimate_transaction_fee(
            rpc_client,
            self,
            Some(&fee_payer),
            config.validation.is_payment_required(),
        )
        .await?;

        let required_lamports = config
            .validation
            .price
            .get_required_lamports(
                Some(rpc_client),
                Some(config.validation.price_source.clone()),
                min_transaction_fee,
            )
            .await?;

        // Only validate payment if not free
        if required_lamports > 0 {
            // Get the expected payment destination
            let payment_destination = config.kora.get_payment_address()?;

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
        self.sign_transaction(rpc_client).await
    }

    async fn sign_and_send_transaction(
        &mut self,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError> {
        let (transaction, encoded) = self.sign_transaction(rpc_client).await?;

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
    use crate::{tests::common::get_mock_rpc_client, transaction::TransactionUtil};

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

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses() {
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

        let rpc_client = get_mock_rpc_client(&Account {
            data: serialized_data,
            executable: false,
            lamports: 0,
            owner: Pubkey::new_unique(),
            rent_epoch: 0,
        });

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
}
