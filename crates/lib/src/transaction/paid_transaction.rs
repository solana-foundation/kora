use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};

use crate::{
    config::ValidationConfig,
    constant::SOL_MINT,
    error::KoraError,
    get_signer,
    oracle::OracleClient,
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

    let oracle_client = OracleClient::new(rpc_client);

    let sol_mint = Pubkey::from_str_const(SOL_MINT);

    let token_price_info = oracle_client
        .get_token_price(&sol_mint)
        .await
        .map(|price| TokenPriceInfo { price })
        .unwrap_or(TokenPriceInfo { price: 0.0 });

    // Validate token payment
    validate_token_payment(
        rpc_client,
        &transaction,
        validation,
        required_lamports,
        signer.solana_pubkey(),
        &token_price_info,
    )
    .await?;

    // Sign the transaction
    sign_transaction(rpc_client, validation, transaction).await
}
