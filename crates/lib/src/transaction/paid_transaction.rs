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
) -> Result<(VersionedTransaction, String), KoraError> {
    let signer = get_signer()?;

    // Get the simulation result for fee calculation
    let min_transaction_fee = estimate_transaction_fee(rpc_client, resolved_transaction).await?;

    let required_lamports = validation
        .price
        .get_required_lamports(
            Some(rpc_client),
            Some(validation.price_source.clone()),
            min_transaction_fee,
        )
        .await?;

    // Only validate payment if not free
    if required_lamports > 0 {
        // Validate token payment
        validate_token_payment(
            resolved_transaction,
            required_lamports,
            validation,
            rpc_client,
            signer.solana_pubkey(),
        )
        .await?;
    }

    let transaction = resolved_transaction.get_transaction().clone();

    // Sign the transaction
    sign_transaction(rpc_client, validation, transaction).await
}
