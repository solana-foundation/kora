use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_keychain::SolanaSigner;
use solana_message::Message;
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey};
use solana_system_interface::instruction::transfer;
use std::{str::FromStr, sync::Arc};
use utoipa::ToSchema;

use crate::{
    constant::NATIVE_SOL,
    state::get_request_signer_with_signer_key,
    transaction::{
        TransactionUtil, VersionedMessageExt, VersionedTransactionOps, VersionedTransactionResolved,
    },
    validator::transaction_validator::TransactionValidator,
    CacheUtil, KoraError,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferTransactionRequest {
    pub amount: u64,
    pub token: String,
    pub source: String,
    pub destination: String,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TransferTransactionResponse {
    pub transaction: String,
    pub message: String,
    pub blockhash: String,
    /// Public key of the signer used (for client consistency)
    pub signer_pubkey: String,
}

pub async fn transfer_transaction(
    rpc_client: &Arc<RpcClient>,
    request: TransferTransactionRequest,
) -> Result<TransferTransactionResponse, KoraError> {
    let signer = get_request_signer_with_signer_key(request.signer_key.as_deref())?;
    let fee_payer = signer.pubkey();

    let validator = TransactionValidator::new(fee_payer)?;

    let source = Pubkey::from_str(&request.source)
        .map_err(|e| KoraError::ValidationError(format!("Invalid source address: {e}")))?;
    let destination = Pubkey::from_str(&request.destination)
        .map_err(|e| KoraError::ValidationError(format!("Invalid destination address: {e}")))?;
    let token_mint = Pubkey::from_str(&request.token)
        .map_err(|e| KoraError::ValidationError(format!("Invalid token address: {e}")))?;

    // manually check disallowed account because we're creating the message
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
        let token_mint = validator.fetch_and_validate_token_mint(&token_mint, rpc_client).await?;
        let token_program = token_mint.get_token_program();
        let decimals = token_mint.decimals();

        let source_ata = token_program.get_associated_token_address(&source, &token_mint.address());
        let dest_ata =
            token_program.get_associated_token_address(&destination, &token_mint.address());

        CacheUtil::get_account(rpc_client, &source_ata, false)
            .await
            .map_err(|_| KoraError::AccountNotFound(source_ata.to_string()))?;

        if CacheUtil::get_account(rpc_client, &dest_ata, false).await.is_err() {
            instructions.push(token_program.create_associated_token_account_instruction(
                &fee_payer,
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

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).await?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&fee_payer),
        &blockhash.0,
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    let mut resolved_transaction =
        VersionedTransactionResolved::from_kora_built_transaction(&transaction)?;

    // validate transaction before signing
    validator.validate_transaction(&mut resolved_transaction, rpc_client).await?;

    // Find the fee payer position in the account keys
    let fee_payer_position = resolved_transaction.find_signer_position(&fee_payer)?;

    let message_bytes = resolved_transaction.transaction.message.serialize();
    let signature = signer
        .sign_message(&message_bytes)
        .await
        .map_err(|e| KoraError::SigningError(e.to_string()))?;

    resolved_transaction.transaction.signatures[fee_payer_position] = signature;

    let encoded = resolved_transaction.encode_b64_transaction()?;
    let message_encoded = transaction.message.encode_b64_message()?;

    emit_event(WebhookEvent::TransactionSigned(TransactionSignedData {
        transaction_id: encoded.clone(),
        signer_pubkey: fee_payer.to_string(),
        method: "transferTransaction".to_string(),
    }))
    .await;

    Ok(TransferTransactionResponse {
        transaction: encoded,
        message: message_encoded,
        blockhash: blockhash.0.to_string(),
        signer_pubkey: fee_payer.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::update_config,
        tests::{
            common::{setup_or_get_test_signer, RpcMockBuilder},
            config_mock::ConfigMockBuilder,
        },
    };

    #[tokio::test]
    async fn test_transfer_transaction_invalid_source() {
        let config = ConfigMockBuilder::new().build();
        update_config(config).unwrap();
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
    async fn test_transfer_transaction_invalid_destination() {
        let config = ConfigMockBuilder::new().build();
        update_config(config).unwrap();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().with_mint_account(6).build());

        let request = TransferTransactionRequest {
            amount: 1000,
            token: Pubkey::new_unique().to_string(),
            source: Pubkey::new_unique().to_string(),
            destination: "invalid_pubkey".to_string(),
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
    async fn test_transfer_transaction_invalid_token() {
        let config = ConfigMockBuilder::new().build();
        update_config(config).unwrap();
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
