use std::{str::FromStr, sync::Arc};

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{message::Message, program_pack::Pack, pubkey::Pubkey, transaction::Transaction};
use spl_token::state::Mint;

use crate::common::{
    config::ValidationConfig, get_signer, validation::TransactionValidator, KoraError, Signer as _,
    JUPITER_API_URL, SOL_MINT,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapToSolRequest {
    pub account: String,
    pub amount: u64,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapToSolResponse {
    pub signature: String,
    pub transaction: String,
}

pub async fn swap_to_sol(
    rpc_client: &Arc<RpcClient>,
    validation: &ValidationConfig,
    request: SwapToSolRequest,
) -> Result<SwapToSolResponse, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    let validator = TransactionValidator::new(signer.solana_pubkey(), validation)?;

    let mint_pubkey = Pubkey::from_str(&request.token)
        .map_err(|_| KoraError::InvalidTransaction("Invalid token mint address".to_string()))?;

    validator.validate_token_mint(&mint_pubkey)?;

    let mint_account = rpc_client
        .get_account(&mint_pubkey)
        .await
        .map_err(|_| KoraError::InvalidTransaction("Failed to fetch mint account".to_string()))?;

    let mint = Mint::unpack(&mint_account.data)
        .map_err(|_| KoraError::InvalidTransaction("Invalid mint account data".to_string()))?;

    let decimals = mint.decimals;
    let amount_with_decimals = request
        .amount
        .checked_mul(10u64.pow(decimals as u32))
        .ok_or(KoraError::InvalidTransaction("Amount overflow".to_string()))?;

    let jupiter_client = JupiterSwapApiClient::new(JUPITER_API_URL.to_string());

    let quote_request = QuoteRequest {
        amount: amount_with_decimals,
        input_mint: mint_pubkey,
        output_mint: Pubkey::from_str(SOL_MINT).unwrap(),
        slippage_bps: 20,
        ..QuoteRequest::default()
    };

    let quote_response = jupiter_client
        .quote(&quote_request)
        .await
        .map_err(|e| KoraError::SwapError(format!("Failed to get quote: {}", e)))?;

    let swap_request = SwapRequest {
        user_public_key: signer.solana_pubkey(),
        quote_response: quote_response.clone(),
        config: TransactionConfig::default(),
    };

    let swap_response = jupiter_client
        .swap_instructions(&swap_request)
        .await
        .map_err(|e| KoraError::SwapError(format!("Failed to get swap instructions: {}", e)))?;

    let mut instructions = Vec::new();
    instructions.extend(swap_response.setup_instructions);
    instructions.push(swap_response.swap_instruction);
    instructions.extend(swap_response.cleanup_instruction);

    // Create and sign the transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| KoraError::SwapError(format!("Failed to get recent blockhash: {}", e)))?;

    let message = Message::new_with_blockhash(
        &instructions,
        Some(&signer.solana_pubkey()),
        &recent_blockhash,
    );

    let mut transaction = Transaction::new_unsigned(message);

    let signature = signer
        .partial_sign_solana(&transaction.message_data())
        .map_err(|e| KoraError::SwapError(format!("Failed to sign transaction: {}", e)))?;

    transaction.signatures = vec![signature];

    let serialized = bincode::serialize(&transaction)
        .map_err(|e| KoraError::SwapError(format!("Failed to serialize transaction: {}", e)))?;

    Ok(SwapToSolResponse {
        signature: signature.to_string(),
        transaction: bs58::encode(serialized).into_string(),
    })
}
