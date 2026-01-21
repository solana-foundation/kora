use crate::{
    bundle::{BundleError, JitoError},
    config::Config,
    constant::ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION,
    fee::fee::{FeeConfigUtil, TransactionFeeUtil},
    signer::bundle_signer::BundleSigner,
    token::token::TokenUtil,
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
    pub total_payment_lamports: u64,
    pub total_solana_estimated_fee: u64,
}

pub enum BundleProcessingMode<'a> {
    CheckUsage(Option<&'a str>),
    SkipUsage,
}

impl BundleProcessor {
    /// Extract transactions at specified indices for processing.
    /// Returns (filtered_transactions, index_to_position_map).
    /// If `sign_only_indices` is None, returns all transactions with all indices.
    pub fn extract_transactions_to_process(
        transactions: &[String],
        sign_only_indices: Option<Vec<usize>>,
    ) -> Result<(Vec<String>, HashMap<usize, usize>), KoraError> {
        let indices = sign_only_indices.unwrap_or_else(|| (0..transactions.len()).collect());

        // Build map and filtered list (duplicates silently ignored)
        let mut index_to_position: HashMap<usize, usize> = HashMap::with_capacity(indices.len());
        let mut filtered: Vec<String> = Vec::with_capacity(indices.len());

        for idx in indices {
            if index_to_position.contains_key(&idx) {
                continue; // skip duplicate
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
    pub async fn process_bundle<'a>(
        encoded_txs: &[String],
        fee_payer: Pubkey,
        payment_destination: &Pubkey,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        sig_verify: bool,
        processing_mode: BundleProcessingMode<'a>,
    ) -> Result<Self, KoraError> {
        let validator = TransactionValidator::new(config, fee_payer)?;
        let mut resolved_transactions = Vec::with_capacity(encoded_txs.len());
        let mut total_required_lamports = 0u64;
        let mut all_bundle_instructions: Vec<Instruction> = Vec::new();
        let mut txs_missing_payment_count = 0u64;

        // Phase 1: Decode, resolve, validate, calc fees, collect instructions
        for encoded in encoded_txs {
            let transaction = TransactionUtil::decode_b64_transaction(encoded)?;

            let mut resolved_tx = VersionedTransactionResolved::from_transaction(
                &transaction,
                config,
                rpc_client,
                sig_verify,
            )
            .await?;

            // Check usage limit for each transaction in the bundle (skip for estimates)
            if let BundleProcessingMode::CheckUsage(user_id) = processing_mode {
                UsageTracker::check_transaction_usage_limit(
                    config,
                    &mut resolved_tx,
                    user_id,
                    &fee_payer,
                    rpc_client,
                )
                .await?;
            }

            validator.validate_transaction(config, &mut resolved_tx, rpc_client).await?;

            let fee_calc = FeeConfigUtil::estimate_kora_fee(
                &mut resolved_tx,
                &fee_payer,
                config.validation.is_payment_required(),
                rpc_client,
                config,
            )
            .await?;

            total_required_lamports =
                total_required_lamports.checked_add(fee_calc.total_fee_lamports).ok_or_else(
                    || KoraError::ValidationError("Bundle fee calculation overflow".to_string()),
                )?;

            // Track how many transactions are missing payment instructions
            if fee_calc.payment_instruction_fee > 0 {
                txs_missing_payment_count += 1;
            }

            all_bundle_instructions.extend(resolved_tx.all_instructions.clone());
            resolved_transactions.push(resolved_tx);
        }

        // For bundles, only ONE payment instruction is needed across all transactions.
        // If multiple transactions are missing payments, we've overcounted by
        // (txs_missing_payment_count - 1) * ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION
        if txs_missing_payment_count > 1 {
            let overcount =
                (txs_missing_payment_count - 1) * ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION;

            total_required_lamports =
                total_required_lamports.checked_sub(overcount).ok_or_else(|| {
                    KoraError::ValidationError("Bundle fee calculation overflow".to_string())
                })?;
        }

        // Phase 2: Calculate payments with cross-tx ATA visibility
        let mut total_payment_lamports = 0u64;
        let mut total_solana_estimated_fee = 0u64;
        for resolved in resolved_transactions.iter_mut() {
            if let Some(payment) = TokenUtil::find_payment_in_transaction(
                config,
                resolved,
                rpc_client,
                payment_destination,
                Some(&all_bundle_instructions),
            )
            .await?
            {
                total_payment_lamports =
                    total_payment_lamports.checked_add(payment).ok_or_else(|| {
                        KoraError::ValidationError("Payment calculation overflow".to_string())
                    })?;
            }

            let fee = TransactionFeeUtil::get_estimate_fee_resolved(rpc_client, resolved).await?;
            total_solana_estimated_fee =
                total_solana_estimated_fee.checked_add(fee).ok_or_else(|| {
                    KoraError::ValidationError("Bundle Solana fee calculation overflow".to_string())
                })?;

            validator.validate_lamport_fee(total_solana_estimated_fee)?;
        }

        Ok(Self {
            resolved_transactions,
            total_required_lamports,
            total_payment_lamports,
            total_solana_estimated_fee,
        })
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
    ) -> Result<Vec<VersionedTransactionResolved>, KoraError> {
        self.validate_payment()?;

        let mut blockhash = None;

        for resolved in self.resolved_transactions.iter_mut() {
            // Get latest blockhash if signatures are empty and blockhash is not set
            if blockhash.is_none() && resolved.transaction.signatures.is_empty() {
                blockhash = Some(
                    rpc_client
                        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                        .await?
                        .0,
                );
            }

            BundleSigner::sign_transaction_for_bundle(resolved, signer, fee_payer, &blockhash)
                .await?;
        }

        Ok(self.resolved_transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_payment_sufficient() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1000,
            total_payment_lamports: 1500,
            total_solana_estimated_fee: 1000,
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_exact() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1000,
            total_payment_lamports: 1000,
            total_solana_estimated_fee: 1000,
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_insufficient() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 2000,
            total_payment_lamports: 1000,
            total_solana_estimated_fee: 1000,
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
            total_payment_lamports: 0,
            total_solana_estimated_fee: 1000,
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_max_values() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: u64::MAX,
            total_payment_lamports: u64::MAX,
            total_solana_estimated_fee: 1000,
        };

        assert!(processor.validate_payment().is_ok());
    }

    #[test]
    fn test_validate_payment_one_lamport_short() {
        let processor = BundleProcessor {
            resolved_transactions: vec![],
            total_required_lamports: 1001,
            total_payment_lamports: 1000,
            total_solana_estimated_fee: 500,
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
            total_payment_lamports: 6000,
            total_solana_estimated_fee: 2500,
        };

        assert_eq!(processor.total_required_lamports, 5000);
        assert_eq!(processor.total_payment_lamports, 6000);
        assert_eq!(processor.total_solana_estimated_fee, 2500);
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
        // Duplicates are silently skipped, only unique indices processed
        assert_eq!(result, vec!["tx0".to_string(), "tx1".to_string()]);
        assert_eq!(index_to_position.len(), 2);
        assert_eq!(index_to_position.get(&0), Some(&0)); // tx0 at position 0 in filtered
        assert_eq!(index_to_position.get(&1), Some(&1)); // tx1 at position 1 in filtered
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
        // indices [2, 0] means: signed[0] = tx2, signed[1] = tx0
        let signed = vec!["signed_tx2".to_string(), "signed_tx0".to_string()];
        // index 2 -> position 0, index 0 -> position 1
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
