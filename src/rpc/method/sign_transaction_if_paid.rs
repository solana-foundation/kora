use crate::common::{
    config::ValidationConfig, get_signer, transaction::decode_b58_transaction,
    validation::TransactionValidator, KoraError, Signer, LAMPORTS_PER_SIGNATURE,
    MIN_BALANCE_FOR_RENT_EXEMPTION,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, program_pack::Pack,
    pubkey::Pubkey,
};
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::state::{Account as TokenAccount, Mint};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignTransactionIfPaidRequest {
    pub transaction: String,
    pub cost_in_lamports: Option<u64>,
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
    cost_in_lamports: u64,
    margin: f64,
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
    validator.validate_transaction(&original_transaction)?;

    let total_required_lamports = calculate_required_lamports(&original_transaction);
    let pricing_params = PricingParams {
        cost_in_lamports: request.cost_in_lamports.unwrap_or(LAMPORTS_PER_SIGNATURE),
        margin: request.margin.unwrap_or(0.0),
    };
    let token_price_info = request.token_price_info.unwrap_or(TokenPriceInfo { price: 0.0 });

    validate_token_payment(
        rpc_client,
        &original_transaction,
        signer_pubkey,
        validation,
        total_required_lamports,
        &pricing_params,
        &token_price_info,
    )
    .await?;

    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| KoraError::Rpc(e.to_string()))?;

    let mut transaction = original_transaction.clone();
    transaction.message.recent_blockhash = blockhash.0;

    let signature = signer.sign_solana(&transaction.message_data()).await?;
    transaction.signatures[0] = signature;

    let encoded = bs58::encode(
        bincode::serialize(&original_transaction)
            .map_err(|e| KoraError::InvalidTransaction(format!("Serialization failed: {}", e)))?,
    )
    .into_string();

    Ok(SignTransactionIfPaidResponse {
        signature: signature.to_string(),
        signed_transaction: encoded,
    })
}

fn calculate_required_lamports(transaction: &solana_sdk::transaction::Transaction) -> u64 {
    let signature_cost = LAMPORTS_PER_SIGNATURE * (transaction.signatures.len() as u64);
    let ata_count = transaction
        .message
        .instructions
        .iter()
        .filter(|ix| {
            transaction.message.account_keys[ix.program_id_index as usize]
                == ASSOCIATED_TOKEN_PROGRAM_ID
        })
        .count();

    signature_cost + (ata_count as u64) * MIN_BALANCE_FOR_RENT_EXEMPTION
}

async fn validate_token_payment(
    rpc_client: &Arc<RpcClient>,
    transaction: &solana_sdk::transaction::Transaction,
    signer_pubkey: Pubkey,
    validation: &ValidationConfig,
    required_lamports: u64,
    pricing_params: &PricingParams,
    price_info: &TokenPriceInfo,
) -> Result<(), KoraError> {
    for ix in transaction.message.instructions.iter() {
        if let Ok(spl_token::instruction::TokenInstruction::Transfer { amount }) =
            spl_token::instruction::TokenInstruction::unpack(&ix.data)
        {
            let dest_pubkey = transaction.message.account_keys[ix.accounts[1] as usize];
            if dest_pubkey != signer_pubkey {
                continue;
            }

            let source_account = rpc_client
                .get_account(&transaction.message.account_keys[ix.accounts[0] as usize])
                .await
                .map_err(|e| KoraError::Rpc(e.to_string()))?;

            let token_data = TokenAccount::unpack(&source_account.data).map_err(|e| {
                KoraError::InvalidTransaction(format!("Invalid token account: {}", e))
            })?;

            if !validation.allowed_spl_paid_tokens.contains(&token_data.mint.to_string()) {
                continue;
            }

            let lamport_value = calculate_token_value_in_lamports(
                amount,
                &token_data.mint,
                rpc_client,
                pricing_params,
                price_info,
            )
            .await?;

            if lamport_value >= required_lamports {
                return Ok(());
            }
        }
    }

    Err(KoraError::InvalidTransaction(format!(
        "Insufficient payment. Required {} lamports",
        required_lamports
    )))
}

async fn calculate_token_value_in_lamports(
    amount: u64,
    mint: &Pubkey,
    rpc_client: &Arc<RpcClient>,
    pricing_params: &PricingParams,
    price_info: &TokenPriceInfo,
) -> Result<u64, KoraError> {
    let mint_data = Mint::unpack(
        &rpc_client.get_account(mint).await.map_err(|e| KoraError::Rpc(e.to_string()))?.data,
    )
    .map_err(|e| KoraError::InvalidTransaction(format!("Invalid mint: {}", e)))?;

    let token_price_per_sig =
        price_info.price * pricing_params.cost_in_lamports as f64 / LAMPORTS_PER_SOL as f64;
    let token_price_with_margin = token_price_per_sig * (1.0 / (1.0 - pricing_params.margin));
    let token_price_in_decimal =
        (token_price_with_margin * (10f64.powi(mint_data.decimals as i32))).floor() as u64 + 1;

    amount
        .checked_mul(LAMPORTS_PER_SOL)
        .and_then(|v| v.checked_div(token_price_in_decimal))
        .ok_or_else(|| KoraError::InvalidTransaction("Token amount calculation error".into()))
}
