use crate::{
    bundle::{BundleError, JitoError},
    config::Config,
    constant::ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION,
    fee::{
        fee::{FeeConfigUtil, TransactionFeeUtil},
        price::PriceModel,
    },
    lighthouse::LighthouseUtil,
    plugin::{PluginExecutionContext, TransactionPluginRunner},
    signer::bundle_signer::BundleSigner,
    token::token::{PaymentLamportTotals, TokenUtil, TransferHookValidationFlow},
    transaction::{TransactionUtil, VersionedTransactionResolved},
    usage_limit::UsageTracker,
    validator::transaction_validator::TransactionValidator,
    KoraError,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};

pub struct BundleProcessor {
    pub resolved_transactions: Vec<VersionedTransactionResolved>,
    pub total_required_lamports: u64,
    pub total_cost_lamports: u64,
    pub total_payment_lamports: u64,
    /// Solana fees of members `0..=i`, so member `i` can assert the fee payer balance it must
    /// keep when it lands on its own.
    pub cumulative_solana_fees: Vec<u64>,
    /// Fee-payer lamport outflow of members `0..=i`, which member `i`'s balance floor must
    /// leave room for. All zero when no Lighthouse assertion is appended.
    pub cumulative_lamport_outflows: Vec<u64>,
}

fn transfer_hook_validation_flow_for_bundle(
    plugin_context: Option<PluginExecutionContext>,
) -> TransferHookValidationFlow {
    match plugin_context {
        Some(PluginExecutionContext::SignBundle) => TransferHookValidationFlow::DelayedSigning,
        _ => TransferHookValidationFlow::ImmediateSignAndSend,
    }
}

impl BundleProcessor {
    /// Extract transactions at specified indices for processing.
    /// Returns (filtered_transactions, index_to_position_map).
    /// If `sign_only_indices` is None, returns all transactions with all indices.
    pub fn extract_transactions_to_process(
        transactions: &[String],
        sign_only_indices: Option<Vec<usize>>,
    ) -> Result<(Vec<String>, HashMap<usize, usize>), KoraError> {
        let mut indices = sign_only_indices.unwrap_or_else(|| (0..transactions.len()).collect());
        indices.sort_unstable();

        let mut index_to_position: HashMap<usize, usize> = HashMap::with_capacity(indices.len());
        let mut filtered: Vec<String> = Vec::with_capacity(indices.len());

        for idx in indices {
            if index_to_position.contains_key(&idx) {
                continue;
            }
            let tx = transactions.get(idx).ok_or_else(|| {
                KoraError::ValidationError(format!(
                    "sign_only_indices index {} out of bounds (bundle has {} transactions)",
                    idx,
                    transactions.len()
                ))
            })?;
            index_to_position.insert(idx, filtered.len());
            filtered.push(tx.clone());
        }

        Ok((filtered, index_to_position))
    }

    /// Merge signed transactions back into the original list, preserving order.
    /// `index_to_position` maps original transaction index -> position in signed_transactions vec.
    pub fn merge_signed_transactions(
        original_transactions: &[String],
        signed_transactions: Vec<String>,
        index_to_position: &std::collections::HashMap<usize, usize>,
    ) -> Vec<String> {
        (0..original_transactions.len())
            .map(|idx| {
                if let Some(&position) = index_to_position.get(&idx) {
                    signed_transactions[position].clone()
                } else {
                    original_transactions[idx].clone()
                }
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn process_bundle(
        encoded_txs: &[String],
        fee_payer: Pubkey,
        payment_destination: &Pubkey,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        sig_verify: bool,
        plugin_context: Option<PluginExecutionContext>,
    ) -> Result<Self, KoraError> {
        let validator = TransactionValidator::new(config, fee_payer)?;
        let plugin_runner = TransactionPluginRunner::from_config(config);
        let transfer_hook_validation_flow =
            transfer_hook_validation_flow_for_bundle(plugin_context);
        let mut resolved_transactions = Vec::with_capacity(encoded_txs.len());
        let mut total_required_lamports = 0u64;
        let mut total_cost_lamports = 0u64;
        let mut all_bundle_instructions: Vec<Instruction> = Vec::new();
        let mut txs_missing_payment_count = 0u64;
        let mut total_transfer_outflow = 0u64;

        let mut alt_cache: HashMap<Pubkey, Vec<Pubkey>> = HashMap::new();

        for encoded in encoded_txs {
            let transaction = TransactionUtil::decode_b64_transaction(encoded)?;

            let mut resolved_tx = VersionedTransactionResolved::from_transaction(
                &transaction,
                config,
                rpc_client,
                sig_verify,
                Some(&mut alt_cache),
            )
            .await?;

            validator.validate_transaction(config, &mut resolved_tx, rpc_client).await?;
            if matches!(plugin_context, Some(PluginExecutionContext::SignAndSendBundle)) {
                resolved_tx.require_verified_signatures_except(&fee_payer)?;
            }
            let outflow =
                validator.calculate_total_outflow(config, &mut resolved_tx, rpc_client).await?;
            total_transfer_outflow =
                total_transfer_outflow.checked_add(outflow).ok_or_else(|| {
                    KoraError::ValidationError("Bundle outflow calculation overflow".to_string())
                })?;
            if total_transfer_outflow > config.validation.max_allowed_lamports {
                return Err(KoraError::InvalidTransaction(format!(
                    "Total bundle transfer amount {} exceeds maximum allowed {}",
                    total_transfer_outflow, config.validation.max_allowed_lamports
                )));
            }
            validator.validate_token2022_transfer_hook_signing_policies(
                config,
                &mut resolved_tx,
                transfer_hook_validation_flow,
            )?;
            if let Some(context) = plugin_context {
                plugin_runner.run(&resolved_tx, config, rpc_client, &fee_payer, context).await?;
            }
            all_bundle_instructions.extend(resolved_tx.all_instructions().iter().cloned());
            resolved_transactions.push(resolved_tx);
        }

        let appends_lighthouse_assertion = config.kora.lighthouse.enabled
            && !matches!(plugin_context, Some(PluginExecutionContext::SignAndSendBundle));

        for resolved_tx in resolved_transactions.iter_mut() {
            let fee_calc = FeeConfigUtil::estimate_kora_fee(
                resolved_tx,
                &fee_payer,
                config.validation.is_payment_required(),
                rpc_client,
                config,
                transfer_hook_validation_flow,
                Some(&all_bundle_instructions),
                appends_lighthouse_assertion,
            )
            .await?;

            total_required_lamports =
                total_required_lamports.checked_add(fee_calc.total_fee_lamports).ok_or_else(
                    || KoraError::ValidationError("Bundle fee calculation overflow".to_string()),
                )?;

            total_cost_lamports =
                total_cost_lamports.checked_add(fee_calc.get_total_fee_lamports()?).ok_or_else(
                    || KoraError::ValidationError("Bundle fee calculation overflow".to_string()),
                )?;

            if fee_calc.payment_instruction_fee > 0 {
                txs_missing_payment_count += 1;
            }
        }

        // In a bundle, only one payment is required for the entire transaction sequence.
        // We adjust the total fee to ensure we don't overcharge for missing payments.
        let overcount = if (txs_missing_payment_count as usize) == resolved_transactions.len()
            && txs_missing_payment_count > 0
        {
            // If every transaction misses a payment we only charge the penalty once since a single payment covers the entire bundle
            (txs_missing_payment_count - 1) * ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION
        } else {
            // If any transaction includes a valid payment we waive the penalty for the entire bundle
            txs_missing_payment_count * ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION
        };

        if overcount > 0 {
            if !matches!(config.validation.price.model, PriceModel::Fixed { .. }) {
                total_required_lamports =
                    total_required_lamports.checked_sub(overcount).ok_or_else(|| {
                        KoraError::ValidationError("Bundle fee calculation overflow".to_string())
                    })?;
            }
            total_cost_lamports = total_cost_lamports.saturating_sub(overcount);
        }

        let mut bundle_payment_totals = PaymentLamportTotals::default();
        let mut total_solana_estimated_fee = 0u64;
        let mut cumulative_solana_fees = Vec::with_capacity(resolved_transactions.len());
        let mut total_lamport_outflow = 0u64;
        let mut cumulative_lamport_outflows = Vec::with_capacity(resolved_transactions.len());
        for resolved in resolved_transactions.iter_mut() {
            let payment_totals = TokenUtil::calculate_payment_lamport_totals(
                config,
                resolved,
                rpc_client,
                payment_destination,
                Some(&all_bundle_instructions),
            )
            .await?;
            bundle_payment_totals.checked_add_assign(payment_totals)?;

            let fee = TransactionFeeUtil::get_estimate_fee_resolved(rpc_client, resolved).await?;
            let assertion_fee = TransactionFeeUtil::get_lighthouse_assertion_fee(
                resolved,
                &fee_payer,
                appends_lighthouse_assertion,
            )?;
            total_solana_estimated_fee = total_solana_estimated_fee
                .checked_add(fee)
                .and_then(|total| total.checked_add(assertion_fee))
                .ok_or_else(|| {
                    KoraError::ValidationError("Bundle Solana fee calculation overflow".to_string())
                })?;

            validator.validate_lamport_fee(total_solana_estimated_fee)?;
            cumulative_solana_fees.push(total_solana_estimated_fee);

            if appends_lighthouse_assertion {
                let outflow = FeeConfigUtil::calculate_fee_payer_lamport_outflow(
                    &fee_payer, resolved, rpc_client, config,
                )
                .await?;
                total_lamport_outflow =
                    total_lamport_outflow.checked_add(outflow).ok_or_else(|| {
                        KoraError::ValidationError(
                            "Bundle lamport outflow calculation overflow".to_string(),
                        )
                    })?;
            }
            cumulative_lamport_outflows.push(total_lamport_outflow);
        }

        let total_payment_lamports = bundle_payment_totals.net_payment();

        Ok(Self {
            resolved_transactions,
            total_required_lamports,
            total_cost_lamports,
            total_payment_lamports,
            cumulative_solana_fees,
            cumulative_lamport_outflows,
        })
    }

    async fn check_usage_limit(
        &mut self,
        config: &Config,
        user_id: Option<&str>,
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        for resolved_tx in self.resolved_transactions.iter_mut() {
            UsageTracker::check_transaction_usage_limit(
                config,
                resolved_tx,
                user_id,
                fee_payer,
                rpc_client,
            )
            .await?;
        }
        Ok(())
    }

    fn validate_payment(&self) -> Result<(), KoraError> {
        if self.total_payment_lamports < self.total_required_lamports {
            return Err(BundleError::Jito(JitoError::InsufficientBundlePayment(
                self.total_required_lamports,
                self.total_payment_lamports,
            ))
            .into());
        }
        Ok(())
    }

    pub async fn sign_all(
        mut self,
        signer: &Arc<solana_keychain::Signer>,
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
        will_send: bool,
        user_id: Option<&str>,
    ) -> Result<Vec<VersionedTransactionResolved>, KoraError> {
        TransactionValidator::validate_strict_pricing(
            config,
            self.total_required_lamports,
            self.total_cost_lamports,
        )?;
        self.validate_payment()?;

        let mut asserted_transactions = Vec::new();
        if config.kora.lighthouse.enabled && !will_send {
            let balance = LighthouseUtil::fetch_fee_payer_balance(rpc_client, fee_payer).await?;
            for ((resolved, cumulative_fee), cumulative_outflow) in self
                .resolved_transactions
                .iter()
                .zip(&self.cumulative_solana_fees)
                .zip(&self.cumulative_lamport_outflows)
            {
                let mut transaction = resolved.transaction.clone();
                LighthouseUtil::add_fee_payer_assertion_for_balance(
                    &mut transaction,
                    fee_payer,
                    balance,
                    *cumulative_fee,
                    *cumulative_outflow,
                    &config.kora.lighthouse,
                )?;
                asserted_transactions.push(transaction);
            }
        }

        self.check_usage_limit(config, user_id, fee_payer, rpc_client).await?;

        for (resolved, transaction) in
            self.resolved_transactions.iter_mut().zip(asserted_transactions)
        {
            resolved.transaction = transaction;
        }

        let mut blockhash = None;
        for resolved in self.resolved_transactions.iter_mut() {
            if blockhash.is_none() && resolved.transaction.signatures.is_empty() {
                blockhash = Some(
                    rpc_client
                        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                        .await?
                        .0,
                );
            }

            BundleSigner::sign_transaction_for_bundle(
                resolved, signer, fee_payer, &blockhash, config,
            )
            .await?;
        }

        Ok(self.resolved_transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_hook_validation_flow_for_bundle_sign_bundle() {
        let flow =
            transfer_hook_validation_flow_for_bundle(Some(PluginExecutionContext::SignBundle));
        assert_eq!(flow, TransferHookValidationFlow::DelayedSigning);
    }

    #[test]
    fn test_transfer_hook_validation_flow_for_bundle_sign_and_send_bundle() {
        let flow = transfer_hook_validation_flow_for_bundle(Some(
            PluginExecutionContext::SignAndSendBundle,
        ));
        assert_eq!(flow, TransferHookValidationFlow::ImmediateSignAndSend);
    }

    #[test]
    fn test_transfer_hook_validation_flow_for_bundle_estimation_context() {
        let flow = transfer_hook_validation_flow_for_bundle(None);
        assert_eq!(flow, TransferHookValidationFlow::ImmediateSignAndSend);
    }

    #[test]
    fn test_bundle_payment_netting_zeroes_cross_transaction_refund_loop() {
        let mut bundle_payment_totals = PaymentLamportTotals::default();
        bundle_payment_totals
            .checked_add_assign(PaymentLamportTotals { inflow: 7_500_000, outflow: 0 })
            .unwrap();
        bundle_payment_totals
            .checked_add_assign(PaymentLamportTotals { inflow: 0, outflow: 7_500_000 })
            .unwrap();

        assert_eq!(bundle_payment_totals.net_payment(), 0);
    }

    #[test]
    fn test_bundle_payment_netting_preserves_positive_residual() {
        let mut bundle_payment_totals = PaymentLamportTotals::default();
        bundle_payment_totals
            .checked_add_assign(PaymentLamportTotals { inflow: 10_000_000, outflow: 0 })
            .unwrap();
        bundle_payment_totals
            .checked_add_assign(PaymentLamportTotals { inflow: 0, outflow: 2_500_000 })
            .unwrap();

        assert_eq!(bundle_payment_totals.net_payment(), 7_500_000);
    }

    fn strict_pricing_config(strict: bool) -> Config {
        crate::tests::config_mock::ConfigMockBuilder::new()
            .with_price_model(crate::fee::price::PriceModel::Fixed {
                amount: 5000,
                token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                strict,
            })
            .build()
    }

    fn processor_with_cost(
        total_required_lamports: u64,
        total_cost_lamports: u64,
    ) -> BundleProcessor {
        BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports,
            total_cost_lamports,
            total_payment_lamports: total_required_lamports,
            cumulative_solana_fees: vec![],
            cumulative_lamport_outflows: vec![],
        }
    }

    async fn sign_empty_bundle(
        processor: BundleProcessor,
        config: &Config,
    ) -> Result<Vec<VersionedTransactionResolved>, KoraError> {
        let keypair = solana_sdk::signature::Keypair::new();
        let signer =
            Arc::new(solana_keychain::Signer::from_memory(&keypair.to_base58_string()).unwrap());
        let rpc_client = crate::tests::rpc_mock::RpcMockBuilder::new().build();
        let fee_payer = solana_sdk::signer::Signer::pubkey(&keypair);
        processor.sign_all(&signer, &fee_payer, &rpc_client, config, false, None).await
    }

    #[tokio::test]
    async fn test_sign_all_rejects_bundle_costing_more_than_strict_fixed_price() {
        let config = strict_pricing_config(true);

        let result = sign_empty_bundle(processor_with_cost(10_000, 10_001), &config).await;

        match result {
            Err(KoraError::ValidationError(msg)) => {
                assert!(msg.contains("Strict pricing violation"), "{msg}")
            }
            Err(other) => panic!("expected strict pricing violation, got {other:?}"),
            Ok(_) => panic!("expected strict pricing violation, bundle was signed"),
        }
    }

    #[tokio::test]
    async fn test_sign_all_accepts_bundle_within_strict_fixed_price() {
        let config = strict_pricing_config(true);

        assert!(sign_empty_bundle(processor_with_cost(10_000, 10_000), &config).await.is_ok());
    }

    #[tokio::test]
    async fn test_sign_all_ignores_cost_when_fixed_price_is_not_strict() {
        let config = strict_pricing_config(false);

        assert!(sign_empty_bundle(processor_with_cost(10_000, 10_001), &config).await.is_ok());
    }

    #[test]
    fn test_validate_payment_sufficient() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1000,
            total_cost_lamports: 0,
            total_payment_lamports: 1500,
            cumulative_solana_fees: vec![1000],
            cumulative_lamport_outflows: vec![0],
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_exact() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1000,
            total_cost_lamports: 0,
            total_payment_lamports: 1000,
            cumulative_solana_fees: vec![1000],
            cumulative_lamport_outflows: vec![0],
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_insufficient() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 2000,
            total_cost_lamports: 0,
            total_payment_lamports: 1000,
            cumulative_solana_fees: vec![1000],
            cumulative_lamport_outflows: vec![0],
        };

        let result = processor.validate_payment();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("insufficient"));
            assert!(msg.contains("2000"));
            assert!(msg.contains("1000"));
        }
    }

    #[test]
    fn test_validate_payment_zero_required() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 0,
            total_cost_lamports: 0,
            total_payment_lamports: 0,
            cumulative_solana_fees: vec![1000],
            cumulative_lamport_outflows: vec![0],
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_max_values() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: u64::MAX,
            total_cost_lamports: 0,
            total_payment_lamports: u64::MAX,
            cumulative_solana_fees: vec![1000],
            cumulative_lamport_outflows: vec![0],
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_one_lamport_short() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1001,
            total_cost_lamports: 0,
            total_payment_lamports: 1000,
            cumulative_solana_fees: vec![500],
            cumulative_lamport_outflows: vec![0],
        };

        let result = processor.validate_payment();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
    }

    #[test]
    fn test_bundle_processor_fields() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 5000,
            total_cost_lamports: 0,
            total_payment_lamports: 6000,
            cumulative_solana_fees: vec![2500],
            cumulative_lamport_outflows: vec![0],
        };

        assert_eq!(processor.total_required_lamports, 5000);
        assert_eq!(processor.total_payment_lamports, 6000);
        assert_eq!(processor.cumulative_solana_fees, vec![2500]);
        assert!(processor.resolved_transactions.is_empty());
    }

    #[test]
    fn test_extract_transactions_none_returns_all() {
        let txs = vec!["tx0".to_string(), "tx1".to_string(), "tx2".to_string()];
        let (result, index_to_position) =
            BundleProcessor::extract_transactions_to_process(&txs, None).unwrap();
        assert_eq!(result, txs);
        assert_eq!(index_to_position.len(), 3);
        assert_eq!(index_to_position.get(&0), Some(&0));
        assert_eq!(index_to_position.get(&1), Some(&1));
        assert_eq!(index_to_position.get(&2), Some(&2));
    }

    #[test]
    fn test_extract_transactions_specific_indices() {
        let txs = vec!["tx0".to_string(), "tx1".to_string(), "tx2".to_string()];
        let (result, index_to_position) =
            BundleProcessor::extract_transactions_to_process(&txs, Some(vec![0, 2])).unwrap();
        assert_eq!(result, vec!["tx0".to_string(), "tx2".to_string()]);
        assert_eq!(index_to_position.len(), 2);
        assert_eq!(index_to_position.get(&0), Some(&0));
        assert_eq!(index_to_position.get(&2), Some(&1));
    }

    #[test]
    fn test_extract_transactions_keeps_bundle_order_for_unordered_indices() {
        let txs = vec!["tx0".to_string(), "tx1".to_string(), "tx2".to_string()];
        let (result, index_to_position) =
            BundleProcessor::extract_transactions_to_process(&txs, Some(vec![2, 0])).unwrap();
        assert_eq!(
            result,
            vec!["tx0".to_string(), "tx2".to_string()],
            "Cumulative Lighthouse allowances are computed in this order, so it must match the \
             order the bundle executes in"
        );
        assert_eq!(index_to_position.get(&0), Some(&0));
        assert_eq!(index_to_position.get(&2), Some(&1));
    }

    #[test]
    fn test_extract_transactions_out_of_bounds() {
        let txs = vec!["tx0".to_string(), "tx1".to_string()];
        let result = BundleProcessor::extract_transactions_to_process(&txs, Some(vec![0, 5]));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[test]
    fn test_extract_transactions_empty_indices() {
        let txs = vec!["tx0".to_string(), "tx1".to_string()];
        let (result, index_to_position) =
            BundleProcessor::extract_transactions_to_process(&txs, Some(vec![])).unwrap();
        assert!(result.is_empty());
        assert!(index_to_position.is_empty());
    }

    #[test]
    fn test_extract_transactions_duplicate_indices_silently_skipped() {
        let txs = vec!["tx0".to_string(), "tx1".to_string()];
        let (result, index_to_position) =
            BundleProcessor::extract_transactions_to_process(&txs, Some(vec![0, 0, 1])).unwrap();
        assert_eq!(result, vec!["tx0".to_string(), "tx1".to_string()]);
        assert_eq!(index_to_position.len(), 2);
        assert_eq!(index_to_position.get(&0), Some(&0));
        assert_eq!(index_to_position.get(&1), Some(&1));
    }

    #[test]
    fn test_merge_signed_transactions_preserves_order() {
        let original =
            vec!["tx0".to_string(), "tx1".to_string(), "tx2".to_string(), "tx3".to_string()];
        let signed = vec!["signed_tx0".to_string(), "signed_tx2".to_string()];
        // index 0 -> position 0, index 2 -> position 1
        let index_to_position =
            std::collections::HashMap::from([(0_usize, 0_usize), (2_usize, 1_usize)]);

        let result =
            BundleProcessor::merge_signed_transactions(&original, signed, &index_to_position);

        assert_eq!(
            result,
            vec![
                "signed_tx0".to_string(),
                "tx1".to_string(),
                "signed_tx2".to_string(),
                "tx3".to_string(),
            ]
        );
    }

    #[test]
    fn test_merge_signed_transactions_all_signed() {
        let original = vec!["tx0".to_string(), "tx1".to_string()];
        let signed = vec!["signed_tx0".to_string(), "signed_tx1".to_string()];
        let index_to_position =
            std::collections::HashMap::from([(0_usize, 0_usize), (1_usize, 1_usize)]);

        let result =
            BundleProcessor::merge_signed_transactions(&original, signed, &index_to_position);
        assert_eq!(result, vec!["signed_tx0".to_string(), "signed_tx1".to_string()]);
    }

    #[test]
    fn test_merge_signed_transactions_descending_indices() {
        let original =
            vec!["tx0".to_string(), "tx1".to_string(), "tx2".to_string(), "tx3".to_string()];
        let signed = vec!["signed_tx2".to_string(), "signed_tx0".to_string()];
        let index_to_position =
            std::collections::HashMap::from([(2_usize, 0_usize), (0_usize, 1_usize)]);

        let result =
            BundleProcessor::merge_signed_transactions(&original, signed, &index_to_position);

        assert_eq!(
            result,
            vec![
                "signed_tx0".to_string(),
                "tx1".to_string(),
                "signed_tx2".to_string(),
                "tx3".to_string(),
            ]
        );
    }
}
