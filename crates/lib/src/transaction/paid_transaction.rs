use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;

use crate::{
    config::ValidationConfig,
    error::KoraError,
    get_signer,
    transaction::{estimate_transaction_fee, validator::validate_token_payment, TokenPriceInfo},
};

use super::transaction::sign_transaction;

pub async fn sign_transaction_if_paid(
    rpc_client: &RpcClient,
    validation: &ValidationConfig,
    transaction: Transaction,
    margin: Option<f64>,
) -> Result<(Transaction, String), KoraError> {
    let signer = get_signer()?;

    // Get the simulation result for fee calculation
    let min_transaction_fee = estimate_transaction_fee(rpc_client, &transaction).await?;

    // Calculate required lamports including the margin
    let margin = margin.unwrap_or(0.0);
    let required_lamports = (min_transaction_fee as f64 * (1.0 + margin)) as u64;

    // Validate token payment
    validate_token_payment(
        rpc_client,
        &transaction,
        validation,
        required_lamports,
        signer.solana_pubkey(),
    )
    .await?;

    // Sign the transaction
    sign_transaction(rpc_client, validation, transaction).await
}
