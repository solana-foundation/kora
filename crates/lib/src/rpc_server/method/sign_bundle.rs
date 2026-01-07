//! Sign bundle RPC method
//!
//! Signs a bundle of transactions for submission to Jito block engine.

use crate::{
    config::JitoConfig,
    jito::{
        add_tip_to_transaction, find_tip_in_bundle,
        tip::validate_tip_amount,
        types::{SignBundleRequest, SignBundleResponse},
    },
    state::{get_config, get_request_signer_with_signer_key},
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
    usage_limit::UsageTracker,
    KoraError,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;

/// Signs a bundle of transactions for Jito submission
pub async fn sign_bundle(
    rpc_client: &Arc<RpcClient>,
    request: SignBundleRequest,
) -> Result<SignBundleResponse, KoraError> {
    let config = get_config()?;
    let jito_config = &config.kora.jito;

    // Validate Jito is enabled
    if !jito_config.enabled {
        return Err(KoraError::ValidationError(
            "Jito bundles are not enabled on this server".to_string(),
        ));
    }

    // Validate bundle size
    validate_bundle_size(&request.transactions, jito_config)?;

    // Decode all transactions
    let mut transactions: Vec<VersionedTransaction> = request
        .transactions
        .iter()
        .map(|tx| TransactionUtil::decode_b64_transaction(tx))
        .collect::<Result<Vec<_>, _>>()?;

    // Check usage limits for all transactions
    for tx in &transactions {
        UsageTracker::check_transaction_usage_limit(config, tx).await?;
    }

    // Get signer
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let fee_payer = signer.pubkey();

    // Find or add tip
    let (tip_lamports, tip_transaction_index) = handle_tip(
        &mut transactions,
        &fee_payer,
        jito_config,
        request.tip_lamports,
        request.auto_add_tip,
    )?;

    // Sign all transactions
    let mut signed_transactions = Vec::with_capacity(transactions.len());
    for tx in transactions {
        let mut resolved = VersionedTransactionResolved::from_transaction(
            &tx, config, rpc_client, true, // sig_verify
        )
        .await?;

        let (signed_tx, _) = resolved.sign_transaction(config, &signer, rpc_client).await?;
        let encoded = TransactionUtil::encode_versioned_transaction(&signed_tx)?;
        signed_transactions.push(encoded);
    }

    Ok(SignBundleResponse {
        signed_transactions,
        signer_pubkey: fee_payer.to_string(),
        tip_lamports,
        tip_transaction_index,
    })
}

/// Validates the bundle size
fn validate_bundle_size(
    transactions: &[String],
    jito_config: &JitoConfig,
) -> Result<(), KoraError> {
    if transactions.is_empty() {
        return Err(KoraError::ValidationError(
            "Bundle must contain at least one transaction".to_string(),
        ));
    }

    if transactions.len() > jito_config.max_transactions_per_bundle {
        return Err(KoraError::ValidationError(format!(
            "Bundle exceeds maximum size: {} transactions (max: {})",
            transactions.len(),
            jito_config.max_transactions_per_bundle
        )));
    }

    Ok(())
}

/// Handles tip detection and addition
fn handle_tip(
    transactions: &mut Vec<VersionedTransaction>,
    fee_payer: &solana_sdk::pubkey::Pubkey,
    jito_config: &JitoConfig,
    requested_tip: Option<u64>,
    auto_add_tip: bool,
) -> Result<(u64, usize), KoraError> {
    // Check for existing tip
    if let Some(tip_info) = find_tip_in_bundle(transactions) {
        // Validate existing tip meets minimum
        validate_tip_amount(tip_info.amount_lamports, jito_config.min_tip_lamports)?;
        return Ok((tip_info.amount_lamports, tip_info.transaction_index));
    }

    // No existing tip found
    if !auto_add_tip {
        return Err(KoraError::ValidationError(
            "Bundle has no tip and auto_add_tip is disabled".to_string(),
        ));
    }

    // Determine tip amount
    let tip_lamports = requested_tip.unwrap_or(jito_config.default_tip_lamports);
    validate_tip_amount(tip_lamports, jito_config.min_tip_lamports)?;

    // Add tip to last transaction
    let last_index = transactions.len() - 1;
    add_tip_to_transaction(&mut transactions[last_index], tip_lamports, fee_payer)?;

    Ok((tip_lamports, last_index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;

    #[tokio::test]
    async fn test_sign_bundle_jito_disabled() {
        // Create config with Jito disabled (default)
        let _m = ConfigMockBuilder::new().build_and_setup();

        let rpc_client = Arc::new(RpcClient::new("http://localhost:8899".to_string()));

        let request = SignBundleRequest {
            transactions: vec!["test".to_string()],
            signer_key: None,
            tip_lamports: None,
            auto_add_tip: true,
        };

        let result = sign_bundle(&rpc_client, request).await;
        assert!(result.is_err());

        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("not enabled"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_validate_bundle_size_empty() {
        let jito_config = JitoConfig::default();
        let result = validate_bundle_size(&[], &jito_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_bundle_size_too_large() {
        let jito_config = JitoConfig { max_transactions_per_bundle: 5, ..Default::default() };

        let transactions: Vec<String> = (0..6).map(|i| format!("tx{}", i)).collect();
        let result = validate_bundle_size(&transactions, &jito_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_bundle_size_valid() {
        let jito_config = JitoConfig { max_transactions_per_bundle: 5, ..Default::default() };

        let transactions: Vec<String> = (0..5).map(|i| format!("tx{}", i)).collect();
        let result = validate_bundle_size(&transactions, &jito_config);
        assert!(result.is_ok());
    }
}
