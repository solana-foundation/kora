use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_commitment_config::CommitmentConfig;
use solana_keychain::{Signer, SolanaSigner};
use solana_message::{
    compiled_instruction::CompiledInstruction, v0::MessageAddressTableLookup, VersionedMessage,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, transaction::VersionedTransaction};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    time::Duration,
};

use solana_transaction_status_client_types::UiTransactionEncoding;

use crate::{
    config::Config,
    error::KoraError,
    fee::fee::{FeeConfigUtil, TransactionFeeUtil},
    lighthouse::LighthouseUtil,
    plugin::{PluginExecutionContext, TransactionPluginRunner},
    sanitize_error,
    state::{get_signer_pool, reserve_request_signer_by_pubkey},
    token::token::TransferHookValidationFlow,
    transaction::{
        instruction_util::IxUtils, ParsedALTInstructionData, ParsedALTInstructionType,
        ParsedBpfLoaderUpgradeableInstructionData, ParsedBpfLoaderUpgradeableInstructionType,
        ParsedLoaderV4InstructionData, ParsedLoaderV4InstructionType, ParsedSPLInstructionData,
        ParsedSPLInstructionType, ParsedSystemInstructionData, ParsedSystemInstructionType,
        Token2022SecurityInstruction, Token2022SecurityParser,
    },
    validator::transaction_validator::TransactionValidator,
    CacheUtil,
};
use solana_address_lookup_table_interface::state::AddressLookupTable;

use super::retry_util::sign_with_retry;

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

    // Parsed ALT instructions by type (None if not parsed yet)
    parsed_alt_instructions:
        Option<HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>>,

    // Parsed Loader-v4 instructions by type (None if not parsed yet)
    parsed_loader_v4_instructions:
        Option<HashMap<ParsedLoaderV4InstructionType, Vec<ParsedLoaderV4InstructionData>>>,

    // Parsed BPF Loader Upgradeable (loader-v3) instructions by type (None if not parsed yet)
    parsed_bpf_loader_upgradeable_instructions: Option<
        HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
    >,

    parsed_token2022_security_instructions: Option<Vec<Token2022SecurityInstruction>>,
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
    fn signer_pubkeys(&self) -> &[Pubkey];
    fn verified_signers(&self) -> HashSet<Pubkey>;
    fn find_signer_position(&self, signer_pubkey: &Pubkey) -> Result<usize, KoraError>;

    async fn sign_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &RpcClient,
        will_send: bool,
    ) -> Result<(VersionedTransaction, String), KoraError>;
    async fn sign_and_send_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError>;
}

impl VersionedTransactionResolved {
    pub async fn from_transaction(
        transaction: &VersionedTransaction,
        config: &Config,
        rpc_client: &RpcClient,
        sig_verify: bool,
    ) -> Result<Self, KoraError> {
        let mut resolved = Self {
            transaction: transaction.clone(),
            all_account_keys: vec![],
            all_instructions: vec![],
            parsed_system_instructions: None,
            parsed_spl_instructions: None,
            parsed_alt_instructions: None,
            parsed_loader_v4_instructions: None,
            parsed_bpf_loader_upgradeable_instructions: None,
            parsed_token2022_security_instructions: None,
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
                    config,
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
            IxUtils::uncompile_instructions(transaction.message.instructions(), &all_account_keys)?;

        let inner_instructions = resolved.fetch_inner_instructions(rpc_client, sig_verify).await?;

        resolved.all_instructions.extend(outer_instructions);
        resolved.all_instructions.extend(inner_instructions);

        Ok(resolved)
    }

    /// Only use this is we built the transaction ourselves, because it won't do any checks for resolving LUT, etc.
    pub fn from_kora_built_transaction(
        transaction: &VersionedTransaction,
    ) -> Result<Self, KoraError> {
        Ok(Self {
            transaction: transaction.clone(),
            all_account_keys: transaction.message.static_account_keys().to_vec(),
            all_instructions: IxUtils::uncompile_instructions(
                transaction.message.instructions(),
                transaction.message.static_account_keys(),
            )?,
            parsed_system_instructions: None,
            parsed_spl_instructions: None,
            parsed_alt_instructions: None,
            parsed_loader_v4_instructions: None,
            parsed_bpf_loader_upgradeable_instructions: None,
            parsed_token2022_security_instructions: None,
        })
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
                    replace_recent_blockhash: false,
                    encoding: Some(UiTransactionEncoding::Base64),
                    accounts: None,
                    min_context_slot: None,
                },
            )
            .await
            .map_err(|e| {
                KoraError::RpcError(format!(
                    "Failed to simulate transaction: {}",
                    sanitize_error!(e)
                ))
            })?;

        if let Some(err) = simulation_result.value.err {
            return Err(KoraError::InvalidTransaction(format!(
                "Transaction simulation failed: {err}"
            )));
        }

        if let Some(inner_instructions) = simulation_result.value.inner_instructions {
            let mut compiled_inner_instructions: Vec<CompiledInstruction> = vec![];
            // Clone so we can extend with CPI-only PDA accounts discovered
            // during inner instruction reconstruction.
            let mut extended_account_keys = self.all_account_keys.clone();
            let mut account_keys_hashmap =
                IxUtils::build_account_keys_hashmap(&extended_account_keys);

            for ix in &inner_instructions {
                for inner_ix in &ix.instructions {
                    let compiled = IxUtils::reconstruct_instruction_from_ui_with_account_key_cache(
                        inner_ix,
                        &mut extended_account_keys,
                        &mut account_keys_hashmap,
                    )?;
                    compiled_inner_instructions.push(compiled);
                }
            }

            self.all_account_keys = extended_account_keys;
            return IxUtils::uncompile_instructions(
                &compiled_inner_instructions,
                &self.all_account_keys,
            );
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

        self.parsed_system_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError("Parsed system instructions not found".to_string())
        })
    }

    pub fn get_or_parse_spl_instructions(
        &mut self,
    ) -> Result<&HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>, KoraError> {
        if self.parsed_spl_instructions.is_none() {
            self.parsed_spl_instructions = Some(IxUtils::parse_token_instructions(self)?);
        }

        self.parsed_spl_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError("Parsed SPL instructions not found".to_string())
        })
    }

    pub fn get_or_parse_alt_instructions(
        &mut self,
    ) -> Result<&HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>, KoraError> {
        if self.parsed_alt_instructions.is_none() {
            self.parsed_alt_instructions = Some(IxUtils::parse_alt_instructions(self)?);
        }

        self.parsed_alt_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError("Parsed ALT instructions not found".to_string())
        })
    }

    pub fn get_or_parse_loader_v4_instructions(
        &mut self,
    ) -> Result<
        &HashMap<ParsedLoaderV4InstructionType, Vec<ParsedLoaderV4InstructionData>>,
        KoraError,
    > {
        if self.parsed_loader_v4_instructions.is_none() {
            self.parsed_loader_v4_instructions = Some(IxUtils::parse_loader_v4_instructions(self)?);
        }

        self.parsed_loader_v4_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError("Parsed Loader-v4 instructions not found".to_string())
        })
    }

    pub fn get_or_parse_bpf_loader_upgradeable_instructions(
        &mut self,
    ) -> Result<
        &HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
        KoraError,
    > {
        if self.parsed_bpf_loader_upgradeable_instructions.is_none() {
            self.parsed_bpf_loader_upgradeable_instructions =
                Some(IxUtils::parse_bpf_loader_upgradeable_instructions(self)?);
        }
        self.parsed_bpf_loader_upgradeable_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError(
                "Parsed BPF Loader Upgradeable instructions not found".to_string(),
            )
        })
    }

    pub fn get_or_parse_token2022_security_instructions(
        &mut self,
    ) -> Result<&Vec<Token2022SecurityInstruction>, KoraError> {
        if self.parsed_token2022_security_instructions.is_none() {
            self.parsed_token2022_security_instructions =
                Some(Token2022SecurityParser::parse(&self.all_instructions)?);
        }

        self.parsed_token2022_security_instructions.as_ref().ok_or_else(|| {
            KoraError::SerializationError(
                "Parsed Token-2022 security instructions not found".to_string(),
            )
        })
    }
}

// Implementation of the consolidated trait for VersionedTransactionResolved
#[async_trait]
impl VersionedTransactionOps for VersionedTransactionResolved {
    fn encode_b64_transaction(&self) -> Result<String, KoraError> {
        let serialized = bincode::serialize(&self.transaction).map_err(|e| {
            KoraError::SerializationError(format!(
                "Base64 serialization failed: {}",
                sanitize_error!(e)
            ))
        })?;
        Ok(STANDARD.encode(serialized))
    }

    fn signer_pubkeys(&self) -> &[Pubkey] {
        let num_required_signatures =
            self.transaction.message.header().num_required_signatures as usize;
        &self.transaction.message.static_account_keys()[..num_required_signatures]
    }

    fn verified_signers(&self) -> HashSet<Pubkey> {
        let message_bytes = self.transaction.message.serialize();

        self.transaction
            .signatures
            .iter()
            .zip(self.signer_pubkeys().iter())
            .filter_map(|(signature, pubkey)| {
                signature.verify(pubkey.as_ref(), &message_bytes).then_some(*pubkey)
            })
            .collect()
    }

    fn find_signer_position(&self, signer_pubkey: &Pubkey) -> Result<usize, KoraError> {
        self.signer_pubkeys().iter().position(|key| key == signer_pubkey).ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Signer {signer_pubkey} not found in transaction signer keys"
            ))
        })
    }

    async fn sign_transaction(
        &mut self,
        config: &Config,
        selected_signer: &std::sync::Arc<Signer>,
        rpc_client: &RpcClient,
        will_send: bool,
    ) -> Result<(VersionedTransaction, String), KoraError> {
        let fee_payer = selected_signer.pubkey();
        let validator = TransactionValidator::new(config, fee_payer)?;

        // Validate transaction and accounts (already resolved)
        validator.validate_transaction(config, self, rpc_client).await?;

        let plugin_context = if will_send {
            PluginExecutionContext::SignAndSendTransaction
        } else {
            PluginExecutionContext::SignTransaction
        };
        TransactionPluginRunner::from_config(config)
            .run(self, config, rpc_client, &fee_payer, plugin_context)
            .await?;

        // Calculate fee and validate payment if price model requires it
        let transfer_hook_validation_flow = if will_send {
            TransferHookValidationFlow::ImmediateSignAndSend
        } else {
            TransferHookValidationFlow::DelayedSigning
        };
        validator.validate_token2022_transfer_hook_signing_policies(
            config,
            self,
            transfer_hook_validation_flow,
        )?;
        let fee_calculation = FeeConfigUtil::estimate_kora_fee(
            self,
            &fee_payer,
            config.validation.is_payment_required(),
            rpc_client,
            config,
            transfer_hook_validation_flow,
        )
        .await?;

        let required_lamports = fee_calculation.total_fee_lamports;

        // Strict fixed pricing must run even when the fixed quote floors to 0 lamports,
        // otherwise a sub-lamport quote would skip the strictness check and sign for free.
        TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calculation)?;

        // Validate payment if price model is not Free
        if required_lamports > 0 {
            log::info!("Payment validation: required_lamports={}", required_lamports);
            // Get the expected payment destination
            let payment_destination = config.kora.get_payment_address(&fee_payer)?;

            // Validate token payment using the resolved transaction
            TransactionValidator::validate_token_payment(
                config,
                self,
                required_lamports,
                rpc_client,
                &payment_destination,
            )
            .await?;
        }

        // Get latest blockhash and update transaction
        let mut transaction = self.transaction.clone();

        if transaction.signatures.is_empty() {
            let blockhash = rpc_client
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .await?;
            transaction.message.set_recent_blockhash(blockhash.0);
        }

        // Validate transaction fee using resolved transaction
        let estimated_fee = TransactionFeeUtil::get_estimate_fee_resolved(rpc_client, self).await?;
        validator.validate_lamport_fee(estimated_fee)?;

        LighthouseUtil::add_fee_payer_assertion(
            &mut transaction,
            rpc_client,
            &fee_payer,
            estimated_fee,
            &config.kora.lighthouse,
            will_send,
        )
        .await?;

        // Sign transaction and report success/failure back to the global signer pool
        // This telemetry ensures unhealthy remote signers are bypassed automatically.
        let message_bytes = transaction.message.serialize();
        let sign_timeout = Duration::from_secs(config.kora.sign_timeout_seconds);
        let max_retries = config.kora.sign_max_retries;
        let signer = reserve_request_signer_by_pubkey(&fee_payer)?;
        let signature =
            match sign_with_retry(sign_timeout, max_retries, "signing", "Signing", || async {
                signer
                    .sign_message(&message_bytes)
                    .await
                    .map_err(|e| KoraError::SigningError(sanitize_error!(e)))
            })
            .await
            {
                Ok(sig) => {
                    // Report success to the pool for health tracking
                    match get_signer_pool() {
                        Ok(pool) => pool.record_signing_success(&signer),
                        Err(e) => log::warn!(
                            "Could not record signing success to pool: {}",
                            sanitize_error!(e)
                        ),
                    }
                    sig
                }
                Err(err) => {
                    // Report failure to the pool to track signer health only after all retries are exhausted.
                    // This prevents a single failing request from immediately blacklisting a signer.
                    match get_signer_pool() {
                        Ok(pool) => pool.record_signing_failure(&signer),
                        Err(pool_err) => log::error!(
                            "Signing failed AND pool health tracking unavailable: {}; \
                         signer failure will not be recorded, automatic failover is disabled",
                            sanitize_error!(pool_err)
                        ),
                    }
                    return Err(err);
                }
            };

        // Find the fee payer position - don't assume it's at position 0
        let fee_payer_position = self.find_signer_position(&fee_payer)?;
        let signatures_len = transaction.signatures.len();
        let signature_slot = match transaction.signatures.get_mut(fee_payer_position) {
            Some(slot) => slot,
            None => {
                return Err(KoraError::InvalidTransaction(format!(
                    "Signer position {fee_payer_position} is out of bounds for signatures (len={signatures_len})"
                )));
            }
        };
        *signature_slot = signature;

        // Serialize signed transaction
        let serialized = bincode::serialize(&transaction)?;
        let encoded = STANDARD.encode(serialized);

        Ok((transaction, encoded))
    }

    async fn sign_and_send_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &RpcClient,
    ) -> Result<(String, String), KoraError> {
        // Payment validation is handled in sign_transaction
        let (transaction, encoded) =
            self.sign_transaction(config, signer, rpc_client, true).await?;

        // Send and confirm transaction
        let signature = rpc_client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| KoraError::RpcError(sanitize_error!(e)))?;

        Ok((signature.to_string(), encoded))
    }
}

pub struct LookupTableUtil {}

impl LookupTableUtil {
    /// Resolves addresses from lookup tables for V0 transactions
    pub async fn resolve_lookup_table_addresses(
        config: &Config,
        rpc_client: &RpcClient,
        lookup_table_lookups: &[MessageAddressTableLookup],
    ) -> Result<Vec<Pubkey>, KoraError> {
        let mut resolved_addresses = Vec::new();

        // Maybe we can use caching here, there's a chance the lookup tables get updated though, so tbd
        for lookup in lookup_table_lookups {
            let lookup_table_account =
                CacheUtil::get_account(config, rpc_client, &lookup.account_key, false)
                    .await
                    .map_err(|e| {
                        KoraError::RpcError(format!(
                            "Failed to fetch lookup table: {}",
                            sanitize_error!(e)
                        ))
                    })?;

            // Parse the lookup table account data to get the actual addresses
            let address_lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data)
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to deserialize lookup table: {}",
                        sanitize_error!(e)
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
    use solana_message::{compiled_instruction::CompiledInstruction, v0, Message};
    use solana_sdk::{
        account::Account,
        hash::Hash,
        instruction::{AccountMeta, Instruction},
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

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();
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

        let resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();
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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let position = transaction.find_signer_position(&keypair.pubkey()).unwrap();
        assert_eq!(position, 0);

        let other_position = transaction.find_signer_position(&other_account);
        assert!(matches!(other_position, Err(KoraError::InvalidTransaction(_))));
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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert_eq!(transaction.find_signer_position(&keypair1.pubkey()).unwrap(), 0);
        assert_eq!(transaction.find_signer_position(&keypair2.pubkey()).unwrap(), 1);
        assert_eq!(transaction.find_signer_position(&keypair3.pubkey()).unwrap(), 2);
    }

    #[test]
    fn test_signer_pubkeys_returns_only_required_signers() {
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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert_eq!(
            transaction.signer_pubkeys(),
            &[keypair1.pubkey(), keypair2.pubkey(), keypair3.pubkey()]
        );
    }

    #[test]
    fn test_verified_signers_excludes_unsigned_signer_slots() {
        let fee_payer = Keypair::new();
        let other_signer = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new_readonly(other_signer.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer.pubkey())));
        let mut transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let message_bytes = transaction.message.serialize();
        let fee_payer_position = transaction
            .message
            .static_account_keys()
            .iter()
            .position(|key| key == &fee_payer.pubkey())
            .unwrap();
        transaction.signatures[fee_payer_position] = fee_payer.sign_message(&message_bytes);

        let resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();
        let verified_signers = resolved.verified_signers();

        assert_eq!(resolved.signer_pubkeys(), &[fee_payer.pubkey(), other_signer.pubkey()]);
        assert!(verified_signers.contains(&fee_payer.pubkey()));
        assert!(!verified_signers.contains(&other_signer.pubkey()));
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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = transaction.find_signer_position(&missing_keypair.pubkey());
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains(&missing_keypair.pubkey().to_string()));
            assert!(msg.contains("not found in transaction signer keys"));
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
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
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
        let message = VersionedMessage::Legacy(Message::new(
            std::slice::from_ref(&instruction),
            Some(&keypair.pubkey()),
        ));
        let transaction = VersionedTransaction::try_new(message.clone(), &[&keypair]).unwrap();

        let resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();

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
        assert!(resolved.parsed_alt_instructions.is_none());
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

        let resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();

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
        let _m = setup_config_mock(config.clone());

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = VersionedMessage::Legacy(Message::new(
            std::slice::from_ref(&instruction),
            Some(&keypair.pubkey()),
        ));
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

        let resolved = VersionedTransactionResolved::from_transaction(
            &transaction,
            &config,
            &rpc_client,
            true,
        )
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
        let _m = setup_config_mock(config.clone());

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

        let resolved = VersionedTransactionResolved::from_transaction(
            &transaction,
            &config,
            &rpc_client,
            true,
        )
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
        let _m = setup_config_mock(config.clone());

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

        let result = VersionedTransactionResolved::from_transaction(
            &transaction,
            &config,
            &rpc_client,
            true,
        )
        .await;

        // The simulation should fail, but the exact error type depends on mock implementation
        // We expect either an RpcError (from mock deserialization) or InvalidTransaction (from simulation logic)
        assert!(result.is_err());

        match result {
            Err(KoraError::RpcError(msg)) => {
                assert!(msg.contains("Failed to simulate transaction"));
            }
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("inner instructions fetching failed"));
            }
            _ => panic!("Expected RpcError or InvalidTransaction"),
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

        let mut resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();
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

        let mut resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();
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
            solana_system_interface::instruction::transfer(&keypair.pubkey(), &recipient, 1000000);
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let mut resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();

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
        let _m = setup_config_mock(config.clone());

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
            LookupTableUtil::resolve_lookup_table_addresses(&config, &rpc_client, &lookups)
                .await
                .unwrap();

        assert_eq!(resolved_addresses.len(), 3);
        assert_eq!(resolved_addresses[0], address1);
        assert_eq!(resolved_addresses[1], address3);
        assert_eq!(resolved_addresses[2], address2);
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_empty() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();
        let lookups = vec![];

        let resolved_addresses =
            LookupTableUtil::resolve_lookup_table_addresses(&config, &rpc_client, &lookups)
                .await
                .unwrap();

        assert_eq!(resolved_addresses.len(), 0);
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_account_not_found() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();
        let lookups = vec![solana_message::v0::MessageAddressTableLookup {
            account_key: Pubkey::new_unique(),
            writable_indexes: vec![0],
            readonly_indexes: vec![],
        }];

        let result =
            LookupTableUtil::resolve_lookup_table_addresses(&config, &rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::RpcError(_))));

        if let Err(KoraError::RpcError(msg)) = result {
            assert!(msg.contains("Failed to fetch lookup table"));
        }
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_invalid_index() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

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

        let result =
            LookupTableUtil::resolve_lookup_table_addresses(&config, &rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("index 1 out of bounds"));
            assert!(msg.contains("writable addresses"));
        }
    }

    #[tokio::test]
    async fn test_resolve_lookup_table_addresses_invalid_readonly_index() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

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

        let result =
            LookupTableUtil::resolve_lookup_table_addresses(&config, &rpc_client, &lookups).await;
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("index 5 out of bounds"));
            assert!(msg.contains("readonly addresses"));
        }
    }
}
