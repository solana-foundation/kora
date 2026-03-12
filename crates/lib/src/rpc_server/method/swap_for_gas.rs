use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    rpc_server::middleware_utils::default_sig_verify,
    state::get_request_signer_with_signer_key,
    swap::{SwapForGasBuildInput, SwapForGasProcessor},
    transaction::TransactionUtil,
};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Request payload for building and Kora-signing a gas-station style swap transaction.
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
}

/// Response payload containing a Kora-signed swap-for-gas transaction.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapForGasResponse {
    /// Base64-encoded transaction signed by Kora fee payer.
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

    let built = SwapForGasProcessor::build_and_sign_transaction(
        &SwapForGasBuildInput {
            source_wallet: request.source_wallet,
            destination_wallet: request.destination_wallet,
            fee_token: request.fee_token,
            lamports_out: request.lamports_out,
        },
        &signer,
        signer_pubkey,
        config,
        rpc_client,
    )
    .await?;

    let encoded = TransactionUtil::encode_versioned_transaction(&built.transaction)?;

    Ok(SwapForGasResponse {
        transaction: encoded,
        signer_pubkey: signer_pubkey.to_string(),
        payment_address: built.payment_address.to_string(),
        fee_token: built.fee_token.to_string(),
        token_amount_in: built.token_amount_in,
        lamports_out: built.lamports_out,
        spread_bps: built.spread_bps,
        destination_wallet: built.destination_wallet.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_for_gas_request_defaults() {
        let json = r#"{
            "source_wallet": "11111111111111111111111111111111",
            "fee_token": "So11111111111111111111111111111111111111112",
            "lamports_out": 5000
        }"#;

        let request: SwapForGasRequest = serde_json::from_str(json).unwrap();
        assert!(!request.sig_verify);
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
        };

        let rpc_client = Arc::new(crate::tests::rpc_mock::RpcMockBuilder::new().build());
        let result = swap_for_gas(&rpc_client, request).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }
}
