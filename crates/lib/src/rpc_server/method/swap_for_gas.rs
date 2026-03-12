use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use solana_message::{Message, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction::transfer as system_transfer;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    rpc_server::middleware_utils::default_sig_verify,
    signer::bundle_signer::BundleSigner,
    state::get_request_signer_with_signer_key,
    swap::get_swap_quote_provider,
    token::token::TokenUtil,
    transaction::{TransactionUtil, VersionedTransactionResolved},
    validator::transaction_validator::TransactionValidator,
    CacheUtil,
};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Request payload for building a gas-station style swap transaction.
///
/// The resulting transaction transfers `fee_token` from `source_wallet` to Kora's payment address,
/// and transfers `lamports_out` SOL from Kora fee payer to `destination_wallet`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapForGasRequest {
    /// Wallet that pays the token side of the swap.
    pub source_wallet: String,
    /// Optional recipient wallet for SOL output (defaults to source_wallet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_wallet: Option<String>,
    /// Mint address of token used for swap payment (for example USDC).
    pub fee_token: String,
    /// Desired SOL output amount in lamports.
    pub lamports_out: u64,
    /// Optional signer selection key for Kora signer consistency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false).
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// If true, Kora signs the fee-payer signature on the built swap transaction.
    #[serde(default)]
    pub sign_swap_transaction: bool,
}

/// Response payload containing a swap-for-gas transaction.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapForGasResponse {
    /// Base64-encoded transaction (unsigned or partially signed by Kora depending on request).
    pub transaction: String,
    /// Public key of the signer used as fee payer.
    pub signer_pubkey: String,
    /// Public key receiving token payments.
    pub payment_address: String,
    /// Mint address of fee token used in swap.
    pub fee_token: String,
    /// Total token amount charged (includes spread).
    pub token_amount_in: u64,
    /// Exact SOL output in lamports.
    pub lamports_out: u64,
    /// Applied spread in basis points.
    pub spread_bps: u16,
    /// SOL recipient wallet.
    pub destination_wallet: String,
    /// True when Kora fee payer signature is present in the returned transaction.
    pub is_signed_by_kora: bool,
}

fn apply_spread_bps(base_amount: u64, spread_bps: u16) -> Result<u64, KoraError> {
    let multiplier = 10_000u128
        .checked_add(spread_bps as u128)
        .ok_or_else(|| KoraError::ValidationError("Spread configuration overflow".to_string()))?;

    let adjusted = (base_amount as u128)
        .checked_mul(multiplier)
        .and_then(|v| v.checked_add(9_999)) // ceil division by 10_000
        .and_then(|v| v.checked_div(10_000))
        .ok_or_else(|| KoraError::ValidationError("Spread calculation overflow".to_string()))?;

    u64::try_from(adjusted)
        .map_err(|_| KoraError::ValidationError("Spread-adjusted amount overflow".to_string()))
}

pub async fn swap_for_gas(
    rpc_client: &Arc<RpcClient>,
    request: SwapForGasRequest,
) -> Result<SwapForGasResponse, KoraError> {
    if request.lamports_out == 0 {
        return Err(KoraError::ValidationError(
            "lamports_out must be greater than zero".to_string(),
        ));
    }

    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let config = &get_config()?;
    let signer_pubkey = signer.pubkey();

    let validator = TransactionValidator::new(config, signer_pubkey)?;

    let source_wallet = Pubkey::from_str(&request.source_wallet)
        .map_err(|e| KoraError::ValidationError(format!("Invalid source_wallet: {e}")))?;
    let destination_wallet = request
        .destination_wallet
        .as_deref()
        .map(Pubkey::from_str)
        .transpose()
        .map_err(|e| KoraError::ValidationError(format!("Invalid destination_wallet: {e}")))?
        .unwrap_or(source_wallet);
    let fee_token = Pubkey::from_str(&request.fee_token)
        .map_err(|e| KoraError::ValidationError(format!("Invalid fee_token mint: {e}")))?;

    if validator.is_disallowed_account(&source_wallet) {
        return Err(KoraError::InvalidTransaction(format!(
            "Source wallet {source_wallet} is disallowed"
        )));
    }
    if validator.is_disallowed_account(&destination_wallet) {
        return Err(KoraError::InvalidTransaction(format!(
            "Destination wallet {destination_wallet} is disallowed"
        )));
    }

    validator.validate_lamport_fee(request.lamports_out)?;

    if !config.validation.supports_token(&fee_token.to_string()) {
        return Err(KoraError::UnsupportedFeeToken(fee_token.to_string()));
    }

    let quote_provider = get_swap_quote_provider(config);
    let quoted_token_amount = quote_provider
        .quote_token_amount_in_for_lamports_out(
            rpc_client,
            &fee_token,
            request.lamports_out,
            config,
        )
        .await?;

    let spread_bps = config.kora.swap_for_gas.spread_bps;
    let token_amount_in = apply_spread_bps(quoted_token_amount, spread_bps)?;

    let payment_destination = config.kora.get_payment_address(&signer_pubkey)?;

    let token_mint = TokenUtil::get_mint(config, rpc_client, &fee_token).await?;
    let token_program = token_mint.get_token_program();
    let token_decimals = token_mint.decimals();

    let source_token_account =
        token_program.get_associated_token_address(&source_wallet, &fee_token);
    let destination_token_account =
        token_program.get_associated_token_address(&payment_destination, &fee_token);

    CacheUtil::get_account(config, rpc_client, &source_token_account, false).await.map_err(
        |e| match e {
            KoraError::AccountNotFound(_) => {
                KoraError::AccountNotFound(source_token_account.to_string())
            }
            other => other,
        },
    )?;

    let mut instructions = Vec::new();

    match CacheUtil::get_account(config, rpc_client, &destination_token_account, false).await {
        Ok(_) => {}
        Err(KoraError::AccountNotFound(_)) => {
            instructions.push(token_program.create_associated_token_account_instruction(
                &signer_pubkey,
                &payment_destination,
                &fee_token,
            ));
        }
        Err(e) => return Err(e),
    }

    instructions.push(
        token_program
            .create_transfer_checked_instruction(
                &source_token_account,
                &fee_token,
                &destination_token_account,
                &source_wallet,
                token_amount_in,
                token_decimals,
            )
            .map_err(|e| {
                KoraError::InvalidTransaction(format!("Failed to build token transfer: {e}"))
            })?,
    );

    instructions.push(system_transfer(&signer_pubkey, &destination_wallet, request.lamports_out));

    let blockhash = CacheUtil::get_or_fetch_latest_blockhash(config, rpc_client).await?;
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&signer_pubkey),
        &blockhash,
    ));

    let mut transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    if request.sign_swap_transaction {
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction)?;
        BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &signer_pubkey,
            &Some(blockhash),
        )
        .await?;
        transaction = resolved.transaction;
    }

    let encoded = TransactionUtil::encode_versioned_transaction(&transaction)?;

    Ok(SwapForGasResponse {
        transaction: encoded,
        signer_pubkey: signer_pubkey.to_string(),
        payment_address: payment_destination.to_string(),
        fee_token: fee_token.to_string(),
        token_amount_in,
        lamports_out: request.lamports_out,
        spread_bps,
        destination_wallet: destination_wallet.to_string(),
        is_signed_by_kora: request.sign_swap_transaction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_spread_bps_zero() {
        let result = apply_spread_bps(1_000_000, 0).unwrap();
        assert_eq!(result, 1_000_000);
    }

    #[test]
    fn test_apply_spread_bps_rounds_up() {
        let result = apply_spread_bps(1, 25).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_swap_for_gas_request_defaults() {
        let json = r#"{
            "source_wallet": "11111111111111111111111111111111",
            "fee_token": "So11111111111111111111111111111111111111112",
            "lamports_out": 5000
        }"#;

        let request: SwapForGasRequest = serde_json::from_str(json).unwrap();
        assert!(!request.sig_verify);
        assert!(!request.sign_swap_transaction);
        assert!(request.destination_wallet.is_none());
    }

    #[tokio::test]
    async fn test_swap_for_gas_rejects_zero_lamports() {
        let request = SwapForGasRequest {
            source_wallet: "11111111111111111111111111111111".to_string(),
            destination_wallet: None,
            fee_token: "So11111111111111111111111111111111111111112".to_string(),
            lamports_out: 0,
            signer_key: None,
            sig_verify: false,
            sign_swap_transaction: false,
        };

        let rpc_client = Arc::new(crate::tests::rpc_mock::RpcMockBuilder::new().build());
        let result = swap_for_gas(&rpc_client, request).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }
}
