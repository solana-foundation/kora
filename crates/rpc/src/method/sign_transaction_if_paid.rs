use super::estimate_transaction_fee::{estimate_transaction_fee, EstimateTransactionFeeRequest};
use kora_lib::{
    config::ValidationConfig, get_signer, transaction::decode_b58_transaction,
    validation::TransactionValidator, KoraError, Signer,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, program_pack::Pack,
    pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::{Account as TokenAccount, Mint};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
    pub margin: Option<f64>,
    pub token_price_info: Option<TokenPriceInfo>,
}

#[derive(Debug, Serialize)]
pub struct SignTransactionIfPaidResponse {
    pub signature: String,
    pub signed_transaction: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenPriceInfo {
    pub price: f64, // Price in SOL
}

#[derive(Debug)]
struct PricingParams {
    margin: u64,
}

pub async fn sign_transaction_if_paid(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SignTransactionIfPaidRequest,
) -> Result<SignTransactionIfPaidResponse, KoraError> {
    let signer = get_signer().map_err(|e| KoraError::SigningError(e.to_string()))?;
    let signer_pubkey = signer.solana_pubkey();

    let original_transaction = decode_b58_transaction(&request.transaction)?;
    let validator = TransactionValidator::new(signer_pubkey, validation)?;
    validator.validate_disallowed_accounts(&original_transaction.message)?;

    // Get the simulation result for fee calculation
    let min_transaction_fee = estimate_transaction_fee(
        rpc_client,
        EstimateTransactionFeeRequest {
            transaction: request.transaction.clone(),
            fee_token: "SOL".to_string(), // or appropriate token
        },
    )
    .await
    .map_err(|e| KoraError::RpcError(e.to_string()))?
    .fee_in_lamports;

    validator.validate_lamport_fee(min_transaction_fee)?;

    let cost_in_lamports = min_transaction_fee;
    let pricing_params = PricingParams { margin: request.margin.unwrap_or(0.0) as u64 };
    let token_price_info = request.token_price_info.unwrap_or(TokenPriceInfo { price: 0.0 });
    // Calculate required lamports including the margin
    let required_lamports = (cost_in_lamports as f64 * (1.0 + pricing_params.margin as f64)) as u64;

    validate_token_payment(
        rpc_client,
        &original_transaction,
        validation,
        required_lamports,
        signer_pubkey,
        &token_price_info,
    )
    .await?;

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    let mut transaction = original_transaction.clone();
    transaction.message.recent_blockhash = blockhash.0;

    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    let encoded = bs58::encode(
        bincode::serialize(&original_transaction)
            .map_err(|e| KoraError::InvalidTransaction(format!("Serialization failed: {}", e)))?,
    )
    .into_string();

    validator.validate_transaction(&transaction)?;

    Ok(SignTransactionIfPaidResponse {
        signature: signature.to_string(),
        signed_transaction: encoded,
    })
}

async fn validate_token_payment(
    rpc_client: &Arc<RpcClient>,
    transaction: &solana_sdk::transaction::Transaction,
    validation: &ValidationConfig,
    required_lamports: u64,
    signer_pubkey: Pubkey,
    price_info: &TokenPriceInfo,
) -> Result<(), KoraError> {
    let mut total_lamport_value = 0;

    for ix in transaction.message.instructions.iter() {
        if *ix.program_id(&transaction.message.account_keys) != spl_token::id() {
            continue;
        }

        if let Ok(spl_token::instruction::TokenInstruction::Transfer { amount }) =
            spl_token::instruction::TokenInstruction::unpack(&ix.data)
        {
            let dest_pubkey = transaction.message.account_keys[ix.accounts[1] as usize];

            let source_key = transaction.message.account_keys[ix.accounts[0] as usize];
            let source_account = rpc_client
                .get_account(&source_key)
                .await
                .map_err(|e| KoraError::RpcError(e.to_string()))?;

            let token_account = spl_token::state::Account::unpack(&source_account.data);

            let mint_pubkey = token_account.unwrap().mint;

            let dest_mint_account = get_associated_token_address(&signer_pubkey, &mint_pubkey);

            if dest_pubkey != dest_mint_account {
                continue;
            }

            if source_account.owner != spl_token::id() {
                continue;
            }

            let token_data = TokenAccount::unpack(&source_account.data).map_err(|e| {
                KoraError::InvalidTransaction(format!("Invalid token account: {}", e))
            })?;

            if token_data.amount < amount {
                continue;
            }

            if !validation.allowed_spl_paid_tokens.contains(&token_data.mint.to_string()) {
                continue;
            }

            let lamport_value =
                calculate_token_value_in_lamports(amount, &token_data.mint, rpc_client, price_info)
                    .await?;

            total_lamport_value += lamport_value;
            if total_lamport_value >= required_lamports {
                return Ok(());
            }
        }
    }

    Err(KoraError::InsufficientFunds(format!(
        "Required {} lamports but only found {} in token transfers",
        required_lamports, total_lamport_value
    )))
}

async fn calculate_token_value_in_lamports(
    amount: u64,
    mint: &Pubkey,
    rpc_client: &Arc<RpcClient>,
    price_info: &TokenPriceInfo,
) -> Result<u64, KoraError> {
    let mint_data = Mint::unpack(
        &rpc_client.get_account(mint).await.map_err(|e| KoraError::RpcError(e.to_string()))?.data,
    )
    .map_err(|e| KoraError::InvalidTransaction(format!("Invalid mint: {}", e)))?;

    let sol_per_token =
        price_info.price * LAMPORTS_PER_SOL as f64 / (10f64.powi(mint_data.decimals as i32));

    let lamport_value = (amount as f64 * sol_per_token).floor() as u64;

    Ok(lamport_value)
}
