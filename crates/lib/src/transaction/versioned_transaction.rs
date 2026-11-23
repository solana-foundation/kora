use async_trait::async_trait;
use serde::Deserialize;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig},
};
use solana_commitment_config::CommitmentConfig;
use solana_keychain::{Signer, SolanaSigner};
use solana_message::compiled_instruction::CompiledInstruction;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Signature,
    transaction::VersionedTransaction,
};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::OnceLock,
};

use solana_transaction_status_client_types::UiTransactionEncoding;

use crate::{
    config::Config,
    error::KoraError,
    fee::fee::{FeeConfigUtil, TransactionFeeUtil},
    lighthouse::LighthouseUtil,
    plugin::{PluginExecutionContext, TransactionPluginRunner},
    sanitize_error,
    state::get_background_tasks,
    token::token::TransferHookValidationFlow,
    transaction::{
        instruction_util::IxUtils, ParsedALTInstructionData, ParsedALTInstructionType,
        ParsedBpfLoaderUpgradeableInstructionData, ParsedBpfLoaderUpgradeableInstructionType,
        ParsedLoaderV4InstructionData, ParsedLoaderV4InstructionType, ParsedSPLInstructionData,
        ParsedSPLInstructionType, ParsedSystemInstructionData, ParsedSystemInstructionType,
        Token2022SecurityInstruction, Token2022SecurityParser, TransactionUtil,
    },
    validator::transaction_validator::TransactionValidator,
};

use super::retry_util::{set_signature_at, sign_with_signer_pool};

fn get_or_try_init<T>(
    cell: &OnceLock<T>,
    init: impl FnOnce() -> Result<T, KoraError>,
) -> Result<&T, KoraError> {
    if let Some(value) = cell.get() {
        return Ok(value);
    }
    let value = init()?;
    Ok(cell.get_or_init(|| value))
}

/// Where an entry of `VersionedTransactionResolved::all_instructions` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionOrigin {
    /// Listed in the message itself.
    TopLevel,
    /// A CPI under top-level `parent_index`, made by `caller_index` (`None` if unknown).
    Inner { parent_index: usize, caller_index: Option<usize> },
}

/// The open calls of one top-level instruction, as `(stack height, all_instructions index)`.
struct CallStack(Vec<(u32, usize)>);

impl CallStack {
    fn new(top_level_index: usize) -> Self {
        Self(vec![(1, top_level_index)])
    }

    /// Records the next inner instruction and returns its direct caller, if known.
    fn enter(&mut self, stack_height: Option<u32>, index: usize) -> Option<usize> {
        // Heights are unreliable from here, so later instructions get no caller.
        let Some(height) = stack_height.filter(|height| *height >= 2) else {
            self.0.clear();
            return None;
        };
        while self.0.last().is_some_and(|(open, _)| *open >= height) {
            self.0.pop();
        }
        let caller =
            self.0.last().filter(|(open, _)| *open == height - 1).map(|(_, caller)| *caller);
        self.0.push((height, index));
        caller
    }
}

/// A transaction with its inner instructions resolved through simulation
pub struct VersionedTransactionResolved {
    pub transaction: VersionedTransaction,

    // Static keys, then the keys only inner instructions reference
    pub all_account_keys: Vec<Pubkey>,

    // Includes all instructions, including inner instructions
    all_instructions: Vec<Instruction>,

    // One entry per `all_instructions` entry; only `new` and `push_inner_instruction` write either
    instruction_origins: Vec<InstructionOrigin>,

    // Parsed instructions by type
    parsed_system_instructions:
        OnceLock<HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>>,

    // Parsed SPL instructions by type
    parsed_spl_instructions:
        OnceLock<HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>>,

    // Parsed ALT instructions by type
    parsed_alt_instructions:
        OnceLock<HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>>,

    // Parsed Loader-v4 instructions by type
    parsed_loader_v4_instructions:
        OnceLock<HashMap<ParsedLoaderV4InstructionType, Vec<ParsedLoaderV4InstructionData>>>,

    // Parsed BPF Loader Upgradeable (loader-v3) instructions by type
    parsed_bpf_loader_upgradeable_instructions: OnceLock<
        HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
    >,

    parsed_token2022_security_instructions: OnceLock<Vec<Token2022SecurityInstruction>>,

    fee_payer_outflow: Option<(Pubkey, i128)>,
}

impl Deref for VersionedTransactionResolved {
    type Target = VersionedTransaction;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

/// The transaction lifecycle milestone `signAndSendTransaction` waits for before
/// responding, ordered by increasing assurance: `Signed` < `Sent` < `Confirmed`.
///
/// All modes share the immediate sign-and-send validation flow (`will_send = true`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RespondAfter {
    /// Wait for on-chain confirmation at `confirmed` commitment (default).
    #[default]
    Confirmed,
    /// Wait only until the RPC node accepts the transaction; no confirmation wait.
    Sent,
    /// Return as soon as signing completes and broadcast in the background. Broadcast
    /// failures are logged server-side, not returned to the caller.
    Signed,
}

#[async_trait]
pub trait VersionedTransactionOps {
    fn encode_b64_transaction(&self) -> Result<String, KoraError>;
    fn signer_pubkeys(&self) -> &[Pubkey];
    fn verified_signers(&self) -> HashSet<Pubkey>;
    fn find_signer_position(&self, signer_pubkey: &Pubkey) -> Result<usize, KoraError>;

    /// Returns the signed transaction, its base64 encoding, and whether a Lighthouse
    /// assertion was appended to the message before signing.
    async fn sign_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &RpcClient,
        will_send: bool,
    ) -> Result<(VersionedTransaction, String, bool), KoraError>;
    async fn sign_and_send_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &std::sync::Arc<RpcClient>,
        respond_after: RespondAfter,
    ) -> Result<(String, String), KoraError>;
}

impl VersionedTransactionResolved {
    pub async fn from_transaction(
        transaction: &VersionedTransaction,
        rpc_client: &RpcClient,
        sig_verify: bool,
    ) -> Result<Self, KoraError> {
        let all_account_keys = transaction.message.static_account_keys().to_vec();

        let outer_instructions =
            IxUtils::uncompile_instructions(transaction.message.instructions(), &all_account_keys)?;

        let mut resolved = Self::new(transaction.clone(), all_account_keys, outer_instructions);
        resolved.fetch_inner_instructions(rpc_client, sig_verify).await?;

        Ok(resolved)
    }

    /// Only use this if we built the transaction ourselves: it skips simulation, so no inner
    /// instructions are resolved.
    pub fn from_kora_built_transaction(
        transaction: &VersionedTransaction,
    ) -> Result<Self, KoraError> {
        let all_account_keys = transaction.message.static_account_keys().to_vec();
        let all_instructions =
            IxUtils::uncompile_instructions(transaction.message.instructions(), &all_account_keys)?;
        Ok(Self::new(transaction.clone(), all_account_keys, all_instructions))
    }

    pub(crate) fn new(
        transaction: VersionedTransaction,
        all_account_keys: Vec<Pubkey>,
        all_instructions: Vec<Instruction>,
    ) -> Self {
        Self {
            transaction,
            all_account_keys,
            instruction_origins: vec![InstructionOrigin::TopLevel; all_instructions.len()],
            all_instructions,
            parsed_system_instructions: OnceLock::new(),
            parsed_spl_instructions: OnceLock::new(),
            parsed_alt_instructions: OnceLock::new(),
            parsed_loader_v4_instructions: OnceLock::new(),
            parsed_bpf_loader_upgradeable_instructions: OnceLock::new(),
            parsed_token2022_security_instructions: OnceLock::new(),
            fee_payer_outflow: None,
        }
    }

    pub(crate) fn cached_fee_payer_outflow(&self, fee_payer: &Pubkey) -> Option<i128> {
        self.fee_payer_outflow
            .filter(|(cached_fee_payer, _)| cached_fee_payer == fee_payer)
            .map(|(_, outflow)| outflow)
    }

    pub(crate) fn cache_fee_payer_outflow(&mut self, fee_payer: Pubkey, outflow: i128) {
        self.fee_payer_outflow = Some((fee_payer, outflow));
    }

    /// Returns the top-level instructions followed by the CPIs simulation surfaced.
    pub fn all_instructions(&self) -> &[Instruction] {
        &self.all_instructions
    }

    /// Returns each entry of `all_instructions` with its origin.
    pub(crate) fn instructions_with_origins(
        &self,
    ) -> impl Iterator<Item = (&Instruction, &InstructionOrigin)> {
        self.all_instructions.iter().zip(&self.instruction_origins)
    }

    /// Appends a CPI surfaced by simulation under the top-level instruction at `parent_index`.
    pub fn push_inner_instruction(
        &mut self,
        parent_index: usize,
        caller_index: Option<usize>,
        instruction: Instruction,
    ) -> Result<(), KoraError> {
        if self.instruction_origins.get(parent_index) != Some(&InstructionOrigin::TopLevel) {
            return Err(KoraError::InvalidTransaction(format!(
                "Inner instruction refers to top-level instruction {parent_index}, which does not exist"
            )));
        }
        self.all_instructions.push(instruction);
        self.instruction_origins.push(InstructionOrigin::Inner { parent_index, caller_index });
        Ok(())
    }

    /// Simulates the transaction and appends every CPI it surfaces to `all_instructions`.
    async fn fetch_inner_instructions(
        &mut self,
        rpc_client: &RpcClient,
        sig_verify: bool,
    ) -> Result<(), KoraError> {
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

        let Some(inner_instructions) = simulation_result.value.inner_instructions else {
            return Ok(());
        };

        let mut compiled_inner_instructions: Vec<CompiledInstruction> = vec![];
        let mut inner_origins: Vec<(usize, Option<usize>)> = vec![];
        // Clone so we can extend with CPI-only PDA accounts discovered
        // during inner instruction reconstruction.
        let mut extended_account_keys = self.all_account_keys.clone();
        let mut account_keys_hashmap = IxUtils::build_account_keys_hashmap(&extended_account_keys);
        let first_inner_index = self.all_instructions.len();

        for ix in &inner_instructions {
            let parent_index = ix.index as usize;
            let mut call_stack = CallStack::new(parent_index);
            for inner_ix in &ix.instructions {
                let compiled = IxUtils::reconstruct_instruction_from_ui_with_account_key_cache(
                    inner_ix,
                    &mut extended_account_keys,
                    &mut account_keys_hashmap,
                )?;
                let index = first_inner_index + compiled_inner_instructions.len();
                let caller_index = call_stack.enter(IxUtils::stack_height(inner_ix), index);
                compiled_inner_instructions.push(compiled);
                inner_origins.push((parent_index, caller_index));
            }
        }

        self.all_account_keys = extended_account_keys;
        let instructions =
            IxUtils::uncompile_instructions(&compiled_inner_instructions, &self.all_account_keys)?;
        for ((parent_index, caller_index), instruction) in
            inner_origins.into_iter().zip(instructions)
        {
            self.push_inner_instruction(parent_index, caller_index, instruction)?;
        }

        Ok(())
    }

    pub fn get_or_parse_system_instructions(
        &self,
    ) -> Result<&HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>, KoraError>
    {
        get_or_try_init(&self.parsed_system_instructions, || {
            IxUtils::parse_system_instructions(self)
        })
    }

    pub fn get_or_parse_spl_instructions(
        &self,
    ) -> Result<&HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>, KoraError> {
        get_or_try_init(&self.parsed_spl_instructions, || IxUtils::parse_token_instructions(self))
    }

    pub fn get_or_parse_alt_instructions(
        &self,
    ) -> Result<&HashMap<ParsedALTInstructionType, Vec<ParsedALTInstructionData>>, KoraError> {
        get_or_try_init(&self.parsed_alt_instructions, || IxUtils::parse_alt_instructions(self))
    }

    pub fn get_or_parse_loader_v4_instructions(
        &self,
    ) -> Result<
        &HashMap<ParsedLoaderV4InstructionType, Vec<ParsedLoaderV4InstructionData>>,
        KoraError,
    > {
        get_or_try_init(&self.parsed_loader_v4_instructions, || {
            IxUtils::parse_loader_v4_instructions(self)
        })
    }

    pub fn get_or_parse_bpf_loader_upgradeable_instructions(
        &self,
    ) -> Result<
        &HashMap<
            ParsedBpfLoaderUpgradeableInstructionType,
            Vec<ParsedBpfLoaderUpgradeableInstructionData>,
        >,
        KoraError,
    > {
        get_or_try_init(&self.parsed_bpf_loader_upgradeable_instructions, || {
            IxUtils::parse_bpf_loader_upgradeable_instructions(self)
        })
    }

    pub fn get_or_parse_token2022_security_instructions(
        &self,
    ) -> Result<&Vec<Token2022SecurityInstruction>, KoraError> {
        get_or_try_init(&self.parsed_token2022_security_instructions, || {
            Token2022SecurityParser::parse(&self.all_instructions)
        })
    }
}

#[async_trait]
impl VersionedTransactionOps for VersionedTransactionResolved {
    fn encode_b64_transaction(&self) -> Result<String, KoraError> {
        TransactionUtil::encode_versioned_transaction(&self.transaction)
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
    ) -> Result<(VersionedTransaction, String, bool), KoraError> {
        let fee_payer = selected_signer.pubkey();
        let validator = TransactionValidator::new(config, fee_payer)?;

        validator.validate_transaction(config, self, rpc_client).await?;

        let plugin_context = if will_send {
            PluginExecutionContext::SignAndSendTransaction
        } else {
            PluginExecutionContext::SignTransaction
        };
        TransactionPluginRunner::from_config(config)
            .run(self, config, rpc_client, &fee_payer, plugin_context)
            .await?;

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
            None,
        )
        .await?;

        let required_lamports = fee_calculation.total_fee_lamports;

        // Strict fixed pricing must run even when the fixed quote floors to 0 lamports,
        // otherwise a sub-lamport quote would skip the strictness check and sign for free.
        TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calculation)?;

        if required_lamports > 0 {
            log::info!("Payment validation: required_lamports={}", required_lamports);
            let payment_destination = config.kora.get_payment_address(&fee_payer)?;

            TransactionValidator::validate_token_payment(
                config,
                self,
                required_lamports,
                rpc_client,
                &payment_destination,
            )
            .await?;
        }

        let mut transaction = self.transaction.clone();

        if transaction.signatures.is_empty() {
            let blockhash = rpc_client
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .await?;
            transaction.message.set_recent_blockhash(blockhash.0);
        }

        let estimated_fee =
            TransactionFeeUtil::get_estimate_fee(rpc_client, &self.transaction.message).await?;
        validator.validate_lamport_fee(estimated_fee)?;

        let lighthouse_assertion_added = LighthouseUtil::add_fee_payer_assertion(
            &mut transaction,
            rpc_client,
            &fee_payer,
            estimated_fee,
            &config.kora.lighthouse,
            will_send,
        )
        .await?;

        let signature = sign_with_signer_pool(
            config,
            &fee_payer,
            &transaction.message.serialize(),
            "signing",
            "Signing",
        )
        .await?;

        // Find the fee payer position - don't assume it's at position 0
        let fee_payer_position = self.find_signer_position(&fee_payer)?;
        set_signature_at(&mut transaction, fee_payer_position, signature)?;

        let encoded = TransactionUtil::encode_versioned_transaction(&transaction)?;

        Ok((transaction, encoded, lighthouse_assertion_added))
    }

    async fn sign_and_send_transaction(
        &mut self,
        config: &Config,
        signer: &std::sync::Arc<Signer>,
        rpc_client: &std::sync::Arc<RpcClient>,
        respond_after: RespondAfter,
    ) -> Result<(String, String), KoraError> {
        // Payment validation is handled in sign_transaction
        let (transaction, encoded, _lighthouse_assertion_added) =
            self.sign_transaction(config, signer, rpc_client, true).await?;

        // Validation already simulated the transaction, so the fast modes skip
        // preflight: a second simulation only delays landing and can fail
        // transiently even though the transaction is valid.
        //
        // That simulation ran before Kora signed and used the caller's sig_verify
        // (default false), so it does not cover signature validity — only the
        // explicit guard below does.
        let skip_preflight_config =
            RpcSendTransactionConfig { skip_preflight: true, ..Default::default() };

        // Skipping preflight also skips the RPC node's signature verification, so a
        // transaction with an unfilled co-signer slot (possible when validation ran
        // with sig_verify off) would be accepted by the RPC but dropped silently by
        // validators. Reject it here so the caller gets an error instead of a
        // signature for a transaction that can never land.
        if transaction.signatures.iter().any(|s| *s == Signature::default()) {
            return Err(KoraError::InvalidTransaction(
                "Transaction is missing required signatures".to_string(),
            ));
        }

        match respond_after {
            RespondAfter::Confirmed => {
                let signature = rpc_client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .map_err(|e| KoraError::RpcError(sanitize_error!(e)))?;

                Ok((signature.to_string(), encoded))
            }
            RespondAfter::Sent => {
                let signature = rpc_client
                    .send_transaction_with_config(&transaction, skip_preflight_config)
                    .await
                    .map_err(|e| KoraError::RpcError(sanitize_error!(e)))?;

                Ok((signature.to_string(), encoded))
            }
            RespondAfter::Signed => {
                // A Solana transaction is identified by its first signature, which is
                // already present once signing completes — so the caller gets it
                // without waiting for the broadcast.
                let signature = transaction
                    .signatures
                    .first()
                    .ok_or_else(|| {
                        KoraError::InvalidTransaction(
                            "Signed transaction has no signatures".to_string(),
                        )
                    })?
                    .to_string();

                // Broadcast in the background so the response returns instantly. This
                // mode carries ZERO delivery guarantee: the response (signature +
                // signed transaction) is returned before the send is even attempted,
                // so any failure — a transient RPC error, the node dropping the
                // transaction, or the broadcast never landing on-chain — happens after
                // the caller already has a "successful" response. Failures are only
                // logged server-side, never returned. Callers who need delivery
                // assurance must use Sent or Confirmed, or rebroadcast the returned
                // signed transaction themselves and verify it landed.
                //
                // The task is registered with the global tracker so a graceful
                // shutdown drains in-flight broadcasts instead of cancelling them
                // when the runtime exits.
                let rpc_client = std::sync::Arc::clone(rpc_client);
                let log_signature = signature.clone();
                get_background_tasks().spawn(async move {
                    if let Err(e) = rpc_client
                        .send_transaction_with_config(&transaction, skip_preflight_config)
                        .await
                    {
                        log::error!(
                            "Background broadcast failed for transaction {log_signature}: {}",
                            sanitize_error!(e)
                        );
                    }
                });

                Ok((signature, encoded))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::SplTokenConfig,
        tests::{
            common::RpcMockBuilder, config_mock::mock_state::setup_config_mock,
            toml_mock::ConfigBuilder, transaction_mock::create_test_message,
        },
        transaction::TransactionUtil,
        Config,
    };

    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use std::collections::HashMap;

    use super::*;

    use solana_message::{compiled_instruction::CompiledInstruction, v1, VersionedMessage};
    use solana_sdk::{
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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let position = transaction.find_signer_position(&keypair.pubkey()).unwrap();
        assert_eq!(position, 0); // Fee payer is typically at position 0
    }

    #[test]
    fn test_find_signer_position_middle_of_accounts() {
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();
        let keypair3 = Keypair::new();
        let program_id = Pubkey::new_unique();

        let v1_message = v1::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 3,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            config: crate::tests::transaction_mock::test_v1_config(),
            account_keys: vec![keypair1.pubkey(), keypair2.pubkey(), keypair3.pubkey(), program_id],
            lifetime_specifier: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 3,
                accounts: vec![0, 1, 2],
                data: vec![1, 2, 3],
            }],
        };
        let message = VersionedMessage::V1(v1_message);
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

        let v1_message = v1::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 3,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            config: crate::tests::transaction_mock::test_v1_config(),
            account_keys: vec![keypair1.pubkey(), keypair2.pubkey(), keypair3.pubkey(), program_id],
            lifetime_specifier: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 3,
                accounts: vec![0, 1, 2],
                data: vec![1, 2, 3],
            }],
        };
        let message = VersionedMessage::V1(v1_message);
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
        let message = create_test_message(&fee_payer.pubkey(), &[instruction]);
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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
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
        let v1_message = v1::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            config: crate::tests::transaction_mock::test_v1_config(),
            account_keys: vec![],
            lifetime_specifier: Hash::default(),
            instructions: vec![],
        };
        let message = VersionedMessage::V1(v1_message);
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
        let message = create_test_message(&keypair.pubkey(), std::slice::from_ref(&instruction));
        let transaction = VersionedTransaction::try_new(message.clone(), &[&keypair]).unwrap();

        let resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();

        assert_eq!(resolved.transaction, transaction);
        assert_eq!(resolved.all_account_keys, transaction.message.static_account_keys());
        assert_eq!(resolved.all_instructions.len(), 1);

        // Compares individual fields, not the whole Instruction, since uncompile_instructions
        // recomputes signer status rather than preserving the original struct.
        let resolved_instruction = &resolved.all_instructions[0];
        assert_eq!(resolved_instruction.program_id, instruction.program_id);
        assert_eq!(resolved_instruction.data, instruction.data);
        assert_eq!(resolved_instruction.accounts.len(), instruction.accounts.len());

        assert!(resolved.parsed_system_instructions.get().is_none());
        assert!(resolved.parsed_spl_instructions.get().is_none());
        assert!(resolved.parsed_alt_instructions.get().is_none());
    }

    #[tokio::test]
    async fn test_from_transaction() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = create_test_message(&keypair.pubkey(), std::slice::from_ref(&instruction));
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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
        // Only the outer instruction: the mock reports no inner instructions.
        assert_eq!(resolved.all_instructions.len(), 1);

        // Compares individual fields, not the whole Instruction, since uncompile_instructions
        // recomputes signer status rather than preserving the original struct.
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
    async fn test_from_transaction_simulation_failure() {
        let config = setup_test_config();
        let _m = setup_config_mock(config.clone());

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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

        // Exact error type depends on mock implementation: RpcError (mock deserialization) or
        // InvalidTransaction (simulation logic).
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
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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
        resolved.fetch_inner_instructions(&rpc_client, true).await.unwrap();

        assert_eq!(resolved.all_instructions.len(), 2);
        assert_eq!(resolved.all_instructions[1].data, vec![10, 20, 30]);
        assert_eq!(
            resolved.instruction_origins,
            vec![
                InstructionOrigin::TopLevel,
                InstructionOrigin::Inner { parent_index: 0, caller_index: None }
            ]
        );
    }

    /// Returns the caller recorded for each simulated inner instruction, given its stack height.
    async fn inner_callers(stack_heights: &[Option<u32>]) -> Vec<Option<usize>> {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let data = bs58::encode(&[1]).into_string();
        let instructions: Vec<_> = stack_heights
            .iter()
            .map(|height| {
                json!({ "programIdIndex": 1, "accounts": [0], "data": data, "stackHeight": height })
            })
            .collect();
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
                    "innerInstructions": [{ "index": 0, "instructions": instructions }]
                }
            }),
        );
        let rpc_client = RpcMockBuilder::new().with_custom_mocks(mocks).build();

        let mut resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();
        resolved.fetch_inner_instructions(&rpc_client, true).await.unwrap();

        resolved.instruction_origins[1..]
            .iter()
            .map(|origin| match origin {
                InstructionOrigin::Inner { parent_index: 0, caller_index } => *caller_index,
                other => panic!("unexpected origin {other:?}"),
            })
            .collect()
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_records_direct_callers() {
        // top(0) -> A(1) -> create(2), then top(0) -> B(3) -> create(4)
        assert_eq!(
            inner_callers(&[Some(2), Some(3), Some(2), Some(3)]).await,
            vec![Some(0), Some(1), Some(0), Some(3)]
        );
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_height_gap_has_no_caller() {
        assert_eq!(inner_callers(&[Some(2), Some(4)]).await, vec![Some(0), None]);
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_missing_height_drops_later_callers() {
        assert_eq!(
            inner_callers(&[Some(2), None, Some(3), Some(2)]).await,
            vec![Some(0), None, None, None]
        );
    }

    #[tokio::test]
    async fn test_fetch_inner_instructions_rejects_parent_index_out_of_range() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

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
                            "index": 1,
                            "instructions": [
                                {
                                    "programIdIndex": 1,
                                    "accounts": [0],
                                    "data": bs58::encode(&[10, 20, 30]).into_string()
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
        let err = resolved.fetch_inner_instructions(&rpc_client, true).await.unwrap_err();
        assert!(
            err.to_string().contains("top-level instruction 1, which does not exist"),
            "unexpected error: {err}"
        );
        assert_eq!(resolved.all_instructions.len(), 1);
    }

    #[tokio::test]
    async fn test_get_or_parse_system_instructions() {
        let config = setup_test_config();
        let _m = setup_config_mock(config);

        let keypair = Keypair::new();
        let recipient = Pubkey::new_unique();

        let instruction =
            solana_system_interface::instruction::transfer(&keypair.pubkey(), &recipient, 1000000);
        let message = create_test_message(&keypair.pubkey(), &[instruction]);
        let transaction = VersionedTransaction::try_new(message, &[&keypair]).unwrap();

        let resolved =
            VersionedTransactionResolved::from_kora_built_transaction(&transaction).unwrap();

        let parsed1_len = {
            let parsed1 = resolved.get_or_parse_system_instructions().unwrap();
            assert!(!parsed1.is_empty());
            parsed1.len()
        };

        let parsed2 = resolved.get_or_parse_system_instructions().unwrap();
        assert_eq!(parsed1_len, parsed2.len());

        assert!(
            parsed2.contains_key(&crate::transaction::ParsedSystemInstructionType::SystemTransfer)
        );
    }
}
