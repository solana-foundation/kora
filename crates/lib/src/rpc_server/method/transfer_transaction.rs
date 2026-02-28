use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use solana_message::Message;
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey};
use solana_system_interface::instruction::transfer;
use std::{str::FromStr, sync::Arc};
use utoipa::ToSchema;

use crate::{
    constant::NATIVE_SOL,
    state::get_request_signer_with_signer_key,
    transaction::{TransactionUtil, VersionedMessageExt},
    validator::transaction_validator::TransactionValidator,
    CacheUtil, KoraError,
};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// **DEPRECATED**: Use `getPaymentInstruction` instead for fee payment flows.
/// This endpoint will be removed in a future version.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferTransactionRequest {
    pub amount: u64,
    pub token: String,
    /// The source wallet address
    pub source: String,
    /// The destination wallet address
    pub destination: String,
    /// Optional signer key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
}

/// **DEPRECATED**: Use `getPaymentInstruction` instead for fee payment flows.
#[derive(Debug, Serialize, ToSchema)]
pub struct TransferTransactionResponse {
    /// Unsigned base64-encoded transaction
    pub transaction: String,
    /// Unsigned base64-encoded message
    pub message: String,
    pub blockhash: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

/// **DEPRECATED**: Use `getPaymentInstruction` instead for fee payment flows.
///
/// Creates an unsigned transfer transaction from source to destination.
/// Kora is the fee payer but does NOT sign - user must sign before submitting.
#[deprecated(since = "2.2.0", note = "Use getPaymentInstruction instead for fee payment flows")]
pub async fn transfer_transaction(
    rpc_client: &Arc<RpcClient>,
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let config = &get_config()?;
    let signer_pubkey = signer.pubkey();

    let validator = TransactionValidator::new(config, signer_pubkey)?;

    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::ValidationError(format!("Invalid source address: {e}")))?;
    let destination = Pubkey::from_str(&request.destination)
        .map_err(|e| KoraError::ValidationError(format!("Invalid destination address: {e}")))?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::ValidationError(format!("Invalid token address: {e}")))?;

    // Check source and destination are not disallowed
    if validator.is_disallowed_account(&source) {
        return Err(KoraError::InvalidTransaction(format!(
            "Source account {source} is disallowed"
        )));
    }
    if validator.is_disallowed_account(&destination) {
        return Err(KoraError::InvalidTransaction(format!(
            "Destination account {destination} is disallowed"
        )));
    }

    let mut instructions = vec![];

    // Handle native SOL transfers
    if request.token == NATIVE_SOL {
        instructions.push(transfer(&source, &destination, request.amount));
    } else {
        // Handle wrapped SOL and other SPL tokens
        let token_mint =
            validator.fetch_and_validate_token_mint(&token_mint, config, rpc_client).await?;
        let token_program = token_mint.get_token_program();
        let decimals = token_mint.decimals();

        let source_ata = token_program.get_associated_token_address(&source, &token_mint.address());
        let dest_ata =
            token_program.get_associated_token_address(&destination, &token_mint.address());

        CacheUtil::get_account(config, rpc_client, &source_ata, false)
            .await
            .map_err(|_| KoraError::AccountNotFound(source_ata.to_string()))?;

        // Create ATA for destination if it doesn't exist (Kora pays for ATA creation)
        if CacheUtil::get_account(config, rpc_client, &dest_ata, false).await.is_err() {
            instructions.push(token_program.create_associated_token_account_instruction(
                &signer_pubkey, // Kora pays for ATA creation
                &destination,
                &token_mint.address(),
            ));
        }

        instructions.push(
            token_program
                .create_transfer_checked_instruction(
                    &source_ata,
                    &token_mint.address(),
                    &dest_ata,
                    &source,
                    request.amount,
                    decimals,
                )
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!(
                        "Failed to create transfer instruction: {e}"
                    ))
                })?,
        );
    }

    let blockhash = CacheUtil::get_or_fetch_latest_blockhash(config, rpc_client).await?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&signer_pubkey), // Kora as fee payer
        &blockhash,
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    let encoded = TransactionUtil::encode_versioned_transaction(&transaction)?;
    let message_encoded = transaction.message.encode_b64_message()?;

    Ok(TransferTransactionResponse {
        transaction: encoded,
        message: message_encoded,
        blockhash: blockhash.to_string(),
        signer_pubkey: signer_pubkey.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        common::{setup_or_get_test_signer, RpcMockBuilder},
        config_mock::ConfigMockBuilder,
    };

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_transfer_transaction_invalid_source() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().with_mint_account(6).build());

        let request = TransferTransactionRequest {
            amount: 1000,
            token: Pubkey::new_unique().to_string(),
            source: "invalid".to_string(),
            destination: Pubkey::new_unique().to_string(),
            signer_key: None,
        };

        let result = transfer_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid source address");
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::ValidationError(_)), "Should return ValidationError");
        match error {
            KoraError::ValidationError(error_message) => {
                assert!(error_message.contains("Invalid source address"));
            }
            _ => panic!("Should return ValidationError"),
        }
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_transfer_transaction_invalid_destination() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().with_mint_account(6).build());

        let request = TransferTransactionRequest {
            amount: 1000,
            token: Pubkey::new_unique().to_string(),
            source: Pubkey::new_unique().to_string(),
            destination: "invalid".to_string(),
            signer_key: None,
        };

        let result = transfer_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid destination address");
        let error = result.unwrap_err();
        match error {
            KoraError::ValidationError(error_message) => {
                assert!(error_message.contains("Invalid destination address"));
            }
            _ => panic!("Should return ValidationError"),
        }
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_transfer_transaction_invalid_token() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().with_mint_account(6).build());

        let request = TransferTransactionRequest {
            amount: 1000,
            token: "invalid_token_address".to_string(),
            source: Pubkey::new_unique().to_string(),
            destination: Pubkey::new_unique().to_string(),
            signer_key: None,
        };

        let result = transfer_transaction(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid token address");
        let error = result.unwrap_err();
        match error {
            KoraError::ValidationError(error_message) => {
                assert!(error_message.contains("Invalid token address"));
            }
            _ => panic!("Should return ValidationError"),
        }
    }
}
