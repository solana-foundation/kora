use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::VersionedTransaction;

use crate::{
    config::ValidationConfig,
    error::KoraError,
    get_signer,
    transaction::{
        estimate_transaction_fee, validator::validate_token_payment, VersionedTransactionExt,
    },
};

use super::transaction::sign_transaction;

pub async fn sign_transaction_if_paid(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    // Should have resolved addresses for lookup tables
    resolved_transaction: &impl VersionedTransactionExt,
    margin: Option<f64>,
) -> Result<(VersionedTransaction, String), KoraError> {
    let signer = get_signer()?;
    let fee_payer = signer.solana_pubkey();

    // Get the simulation result for fee calculation
    let min_transaction_fee =
        estimate_transaction_fee(rpc_client, resolved_transaction, Some(&fee_payer)).await?;

    // Calculate required lamports including the margin
    let margin = margin.unwrap_or(0.0);
    let required_lamports = (min_transaction_fee as f64 * (1.0 + margin)) as u64;

    // Validate token payment
    validate_token_payment(
        resolved_transaction,
        required_lamports,
        validation,
        rpc_client,
        signer.solana_pubkey(),
    )
    .await?;

    let transaction = resolved_transaction.get_transaction().clone();

    // Sign the transaction
    sign_transaction(rpc_client, validation, transaction).await
}
