use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
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
    CacheUtil, KoraError, Signer as _,
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
    log::error!("RPC Method: transferTransaction - Entry: amount={}, token={}, source={}, destination={}, signer_key={:?}",
        request.amount, request.token, request.source, request.destination, request.signer_key);

    let signer = match get_request_signer_with_signer_key(request.signer_key.as_deref()) {
        Ok(s) => {
            log::error!("Signer obtained: pubkey={}", s.solana_pubkey());
            s
        }
        Err(e) => {
            log::error!("Failed to get signer: {e}");
            return Err(e);
        }
    };
    let fee_payer = signer.solana_pubkey();

    log::error!("Creating transaction validator with fee_payer={fee_payer}");
    let validator = match TransactionValidator::new(fee_payer) {
        Ok(v) => {
            log::error!("Transaction validator created successfully");
            v
        }
        Err(e) => {
            log::error!("Failed to create transaction validator: {e}");
            return Err(e);
        }
    };

    log::error!("Parsing addresses from request");
    let source = Pubkey::from_str(&request.source).map_err(|e| {
        log::error!("Invalid source address: {e}");
        KoraError::ValidationError(format!("Invalid source address: {e}"))
    })?;
    let destination = Pubkey::from_str(&request.destination).map_err(|e| {
        log::error!("Invalid destination address: {e}");
        KoraError::ValidationError(format!("Invalid destination address: {e}"))
    })?;
    let token_mint = Pubkey::from_str(&request.token).map_err(|e| {
        log::error!("Invalid token address: {e}");
        KoraError::ValidationError(format!("Invalid token address: {e}"))
    })?;
    log::error!("Addresses parsed successfully: source={source}, destination={destination}, token_mint={token_mint}");

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
        log::error!("Creating native SOL transfer instruction: {} lamports", request.amount);
        instructions.push(transfer(&source, &destination, request.amount));
    } else {
        // Handle wrapped SOL and other SPL tokens
        log::error!("Fetching and validating SPL token mint: {token_mint}");
        let token_mint =
            match validator.fetch_and_validate_token_mint(&token_mint, rpc_client).await {
                Ok(mint) => {
                    log::error!("Token mint validated successfully: decimals={}", mint.decimals());
                    mint
                }
                Err(e) => {
                    log::error!("Failed to fetch/validate token mint: {e}");
                    return Err(e);
                }
            };
        let token_program = token_mint.get_token_program();
        let decimals = token_mint.decimals();

        let source_ata = token_program.get_associated_token_address(&source, &token_mint.address());
        let dest_ata =
            token_program.get_associated_token_address(&destination, &token_mint.address());
        log::error!("Computed ATAs: source_ata={source_ata}, dest_ata={dest_ata}");

        log::error!("Checking if source ATA exists");
        match CacheUtil::get_account(rpc_client, &source_ata, false).await {
            Ok(_) => {
                log::error!("Source ATA exists");
            }
            Err(e) => {
                log::error!("Source ATA not found: {e}");
                return Err(KoraError::AccountNotFound(source_ata.to_string()));
            }
        }

        log::error!("Checking if destination ATA exists");
        if CacheUtil::get_account(rpc_client, &dest_ata, false).await.is_err() {
            log::error!("Destination ATA not found, creating ATA instruction");
            instructions.push(token_program.create_associated_token_account_instruction(
                &fee_payer,
                &destination,
                &token_mint.address(),
            ));
        } else {
            log::error!("Destination ATA already exists");
        }

        log::error!(
            "Creating SPL transfer instruction: amount={}, decimals={}",
            request.amount,
            decimals
        );
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
                    log::error!("Failed to create transfer instruction: {e}");
                    KoraError::InvalidTransaction(format!(
                        "Failed to create transfer instruction: {e}"
                    ))
                })?,
        );
        log::error!("SPL transfer instruction created successfully");
    }

    log::error!("Getting latest blockhash with commitment=confirmed");
    let blockhash = match rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
    {
        Ok(result) => {
            log::error!("Blockhash obtained: {}, context_slot={}", result.0, result.1);
            result
        }
        Err(e) => {
            log::error!("Failed to get blockhash: {e}");
            return Err(e.into());
        }
    };

    log::error!("Creating legacy transaction message with {} instructions", instructions.len());
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&fee_payer),
        &blockhash.0,
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    log::error!("Creating resolved transaction from Kora-built transaction");
    let mut resolved_transaction =
        VersionedTransactionResolved::from_kora_built_transaction(&transaction);

    log::error!("Validating transaction before signing");
    match validator.validate_transaction(&mut resolved_transaction).await {
        Ok(_) => {
            log::error!("Transaction validation successful");
        }
        Err(e) => {
            log::error!("Transaction validation failed: {e}");
            return Err(e);
        }
    }

    log::error!("Finding fee payer position in account keys");
    let fee_payer_position = match resolved_transaction.find_signer_position(&fee_payer) {
        Ok(pos) => {
            log::error!("Fee payer position found: {pos}");
            pos
        }
        Err(e) => {
            log::error!("Failed to find fee payer position: {e}");
            return Err(e);
        }
    };

    log::error!("Signing transaction with signer");
    let signature = match signer.sign_solana(&resolved_transaction).await {
        Ok(sig) => {
            log::error!("Transaction signed successfully");
            sig
        }
        Err(e) => {
            log::error!("Transaction signing failed: {e}");
            return Err(e);
        }
    };

    resolved_transaction.transaction.signatures[fee_payer_position] = signature;

    log::error!("Encoding transaction and message to base64");
    let encoded = match resolved_transaction.encode_b64_transaction() {
        Ok(enc) => {
            log::error!("Transaction encoded successfully");
            enc
        }
        Err(e) => {
            log::error!("Transaction encoding failed: {e}");
            return Err(e);
        }
    };
    let message_encoded = match transaction.message.encode_b64_message() {
        Ok(enc) => {
            log::error!("Message encoded successfully");
            enc
        }
        Err(e) => {
            log::error!("Message encoding failed: {e}");
            return Err(e);
        }
    };

    log::error!(
        "RPC Method: transferTransaction - Success: blockhash={}, signer_pubkey={}",
        blockhash.0,
        fee_payer
    );

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
        let _ = update_config(config);
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = TransferTransactionRequest {
            amount: 1000,
            token: Pubkey::new_unique().to_string(),
            source: "invalid_pubkey".to_string(),
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
        let _ = update_config(config);
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

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
        let _ = update_config(config);
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

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
