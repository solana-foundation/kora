use crate::{
    bundle::{BundleError, BundleProcessingMode, BundleProcessor, JitoError},
    error::KoraError,
    fee::fee::FeeConfigUtil,
    rpc_server::middleware_utils::default_sig_verify,
    state::select_request_signer_with_signer_key,
    validator::bundle_validator::BundleValidator,
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use std::sync::Arc;
use utoipa::ToSchema;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateBundleFeeRequest {
    /// Array of base64-encoded transactions
    pub transactions: Vec<String>,
    #[serde(default)]
    pub fee_token: Option<String>,
    /// Optional signer signer_key to ensure consistency across related RPC calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key: Option<String>,
    /// Whether to verify signatures during simulation (defaults to false)
    #[serde(default = "default_sig_verify")]
    pub sig_verify: bool,
    /// Optional indices of transactions to estimate fees for (defaults to all if not specified)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_only_indices: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateBundleFeeResponse {
    pub fee_in_lamports: u64,
    pub fee_in_token: Option<u64>,
    /// Public key of the signer used for fee estimation (for client consistency)
    pub signer_pubkey: String,
    /// Public key of the payment destination
    pub payment_address: String,
}

pub async fn estimate_bundle_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateBundleFeeRequest,
) -> Result<EstimateBundleFeeResponse, KoraError> {
    let EstimateBundleFeeRequest {
        transactions,
        fee_token,
        signer_key,
        sig_verify,
        sign_only_indices,
    } = request;
    let config = &get_config()?;

    if !config.kora.bundle.enabled {
        return Err(BundleError::Jito(JitoError::NotEnabled).into());
    }

    // Validate bundle size on ALL transactions first
    BundleValidator::validate_jito_bundle_size(&transactions)?;

    // Extract only the transactions we need to process
    let (transactions_to_process, _index_to_position) =
        BundleProcessor::extract_transactions_to_process(&transactions, sign_only_indices.clone())?;

    let signer = select_request_signer_with_signer_key(signer_key.as_deref())?;
    let fee_payer = signer.pubkey();
    let payment_destination = config.kora.get_payment_address(&fee_payer)?;

    let sig_verify = sig_verify || config.kora.force_sig_verify;
    let processor = BundleProcessor::process_bundle(
        &transactions_to_process,
        fee_payer,
        &payment_destination,
        config,
        rpc_client,
        sig_verify,
        None,
        BundleProcessingMode::SkipUsage,
    )
    .await?;

    let fee_in_lamports = processor.total_required_lamports;

    let signed_indices = BundleValidator::signed_indices_for_bundle(
        transactions.len(),
        sign_only_indices.as_deref(),
    );
    BundleValidator::simulate_and_validate_sequential_bundle(
        rpc_client,
        config,
        &transactions,
        &signed_indices,
        &fee_payer,
        true,
    )
    .await?;

    // Calculate fee in token if requested
    let fee_in_token = FeeConfigUtil::calculate_fee_in_token(
        fee_in_lamports,
        fee_token.as_deref(),
        rpc_client,
        config,
    )
    .await?;

    Ok(EstimateBundleFeeResponse {
        fee_in_lamports,
        fee_in_token,
        signer_pubkey: fee_payer.to_string(),
        payment_address: payment_destination.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::TransactionPluginType,
        fee::price::{PriceConfig, PriceModel},
        oracle::PriceSource,
        tests::{
            account_mock::{create_mock_token2022_mint_with_extensions, create_mock_token_account},
            cache_mock::MockCacheUtil,
            common::{setup_or_get_test_signer, setup_or_get_test_usage_limiter, RpcMockBuilder},
            config_mock::{mock_state::setup_config_mock, ConfigMockBuilder},
            transaction_mock::create_mock_encoded_transaction,
        },
        transaction::TransactionUtil,
    };
    use mockito::{Matcher, Server};
    use serde_json::json;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{account::Account, pubkey::Pubkey, signature::Signer};
    use solana_system_interface::instruction::transfer;
    use spl_associated_token_account_interface::{
        address::get_associated_token_address_with_program_id,
        instruction::create_associated_token_account_idempotent,
    };
    use spl_token_2022_interface::extension::ExtensionType;

    #[tokio::test]
    async fn test_estimate_bundle_fee_empty_bundle() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec![],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with empty bundle");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_disabled() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(false).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["some_tx".to_string()],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail when bundles disabled");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("not enabled"));
        }
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_too_large() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["tx".to_string(); 6],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with too many transactions");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::JitoError(_)));
        if let KoraError::JitoError(msg) = err {
            assert!(msg.contains("maximum size"));
        }
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_invalid_signer_key() {
        let _m = ConfigMockBuilder::new().with_bundle_enabled(true).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = Arc::new(RpcMockBuilder::new().build());

        let request = EstimateBundleFeeRequest {
            transactions: vec!["some_tx".to_string()],
            fee_token: None,
            signer_key: Some("invalid_pubkey".to_string()),
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_err(), "Should fail with invalid signer key");
        let err = result.unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_exactly_max_size() {
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build_and_setup();
        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // 5 transactions is the maximum allowed
        let transactions: Vec<String> = (0..5).map(|_| create_mock_encoded_transaction()).collect();

        let request = EstimateBundleFeeRequest {
            transactions,
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transactions");
        let response = result.unwrap();
        assert!(response.fee_in_lamports > 0);
        assert!(!response.signer_pubkey.is_empty());
        assert!(!response.payment_address.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_single_transaction() {
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build_and_setup();
        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // Single transaction bundle is valid
        let request = EstimateBundleFeeRequest {
            transactions: vec![create_mock_encoded_transaction()],
            fee_token: None,
            signer_key: None,
            sig_verify: true,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        assert!(result.is_ok(), "Should succeed with valid transaction");
        let response = result.unwrap();
        assert!(response.fee_in_lamports > 0);
        assert!(!response.signer_pubkey.is_empty());
        assert!(!response.payment_address.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_sig_verify_default() {
        // Test that sig_verify defaults correctly via serde (defaults to false)
        let json = r#"{"transactions": ["tx1"]}"#;
        let request: EstimateBundleFeeRequest = serde_json::from_str(json).unwrap();

        assert!(!request.sig_verify, "sig_verify should default to false");
        assert!(request.signer_key.is_none());
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_request_deserialization() {
        let json = r#"{
            "transactions": ["tx1", "tx2"],
            "fee_token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "signer_key": "11111111111111111111111111111111",
            "sig_verify": false
        }"#;
        let request: EstimateBundleFeeRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.transactions.len(), 2);
        assert_eq!(
            request.fee_token,
            Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string())
        );
        assert_eq!(request.signer_key, Some("11111111111111111111111111111111".to_string()));
        assert!(!request.sig_verify);
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_skips_plugins() {
        let mut config = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .build();
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
        let _m = setup_config_mock(config);

        let _ = setup_or_get_test_signer();
        let _ = setup_or_get_test_usage_limiter().await;

        let rpc_client =
            Arc::new(RpcMockBuilder::new().with_fee_estimate(5000).with_simulation().build());

        // Not gas_swap-compatible shape; would fail if plugins ran during estimate.
        let request = EstimateBundleFeeRequest {
            transactions: vec![create_mock_encoded_transaction()],
            fee_token: None,
            signer_key: None,
            sig_verify: false,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;
        assert!(result.is_ok(), "estimateBundleFee should skip transaction plugins");
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_rejects_sequential_outflow_violation() {
        let mut server = Server::new_async().await;
        let simulate_mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({"method": "simulateBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"value":{"summary":"succeeded","transactionResults":[{"err":null,"logs":["Program 11111111111111111111111111111111 invoke [1]"],"preExecutionAccounts":[{"lamports":2500000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}],"postExecutionAccounts":[{"lamports":1000000,"owner":"11111111111111111111111111111111","data":["","base64"],"executable":false,"rentEpoch":0}]}]}}}"#,
            )
            .create();

        let mut config = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_usage_limit_enabled(false)
            .with_max_allowed_lamports(1_000_000)
            .build();
        config.validation.price = PriceConfig { model: PriceModel::Free };
        config.kora.bundle.jito.block_engine_url = server.url();
        config.kora.bundle.jito.simulate_bundle_url = Some(server.url());
        let _m = setup_config_mock(config);
        let _ = setup_or_get_test_usage_limiter().await;

        let signer_pubkey = setup_or_get_test_signer();
        let rpc_client = Arc::new(
            RpcMockBuilder::new()
                .with_fee_estimate(5000)
                .with_blockhash()
                .with_simulation()
                .build(),
        );

        let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1_000_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&signer_pubkey)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let encoded_tx = TransactionUtil::encode_versioned_transaction(&transaction).unwrap();

        let request = EstimateBundleFeeRequest {
            transactions: vec![encoded_tx],
            fee_token: None,
            signer_key: Some(signer_pubkey.to_string()),
            sig_verify: false,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await;

        simulate_mock.assert();
        assert!(result.is_err(), "Expected sequential outflow validation error");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Total transfer amount"), "Unexpected error: {err}");
        assert!(err.contains("exceeds maximum allowed"), "Unexpected error: {err}");
    }

    #[tokio::test]
    async fn test_estimate_bundle_fee_cross_leg_ata_payment() {
        let _m = ConfigMockBuilder::new()
            .with_bundle_enabled(true)
            .with_cache_enabled(true)
            .with_price_model(PriceModel::Margin { margin: 0.1 })
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                "11111111111111111111111111111111".to_string(), // System Program
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(), // Token Program
                "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string(), // ATA Program
                "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string(), // Token-2022 Program
            ])
            .build_and_setup();

        let fee_payer = setup_or_get_test_signer();
        let mint = Pubkey::new_unique();
        let sender = solana_sdk::signature::Keypair::new();
        let token2022_id = spl_token_2022_interface::id();

        let payment_ata =
            get_associated_token_address_with_program_id(&fee_payer, &mint, &token2022_id);
        let sender_ata =
            get_associated_token_address_with_program_id(&sender.pubkey(), &mint, &token2022_id);

        let mint_account =
            create_mock_token2022_mint_with_extensions(6, vec![ExtensionType::TransferFeeConfig]);
        let sender_ata_account = create_mock_token_account(&sender.pubkey(), &mint);

        // Mock CacheUtil for getting accounts
        let mint_account_clone = mint_account.clone();
        let cache_ctx = MockCacheUtil::get_account_context();
        cache_ctx.expect().returning(move |_, _, addr: &Pubkey, _| {
            if *addr == payment_ata {
                Err(KoraError::AccountNotFound(payment_ata.to_string()))
            } else if *addr == mint {
                Ok(mint_account_clone.clone())
            } else if *addr == sender_ata {
                Ok(sender_ata_account.clone())
            } else {
                Ok(Account::default())
            }
        });

        let rpc_client = Arc::new(
            RpcMockBuilder::new()
                .with_fee_estimate(5000)
                .with_blockhash()
                .with_simulation()
                .with_epoch_info_mock()
                .with_account_info(&mint_account)
                .build(),
        );

        let ata_create_ix = create_associated_token_account_idempotent(
            &sender.pubkey(),
            &fee_payer,
            &mint,
            &token2022_id,
        );
        let msg1 = VersionedMessage::Legacy(Message::new(&[ata_create_ix], Some(&sender.pubkey())));
        let tx1 = TransactionUtil::new_unsigned_versioned_transaction(msg1);
        let encoded_tx1 = TransactionUtil::encode_versioned_transaction(&tx1).unwrap();

        let transfer_amount = 100_000;
        let transfer_ix = spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee(
            &token2022_id,
            &sender_ata,
            &mint,
            &payment_ata,
            &sender.pubkey(),
            &[],
            transfer_amount,
            6,
            0,
        ).unwrap();

        let msg2 = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&sender.pubkey())));
        let tx2 = TransactionUtil::new_unsigned_versioned_transaction(msg2);
        let encoded_tx2 = TransactionUtil::encode_versioned_transaction(&tx2).unwrap();

        let request = EstimateBundleFeeRequest {
            transactions: vec![encoded_tx1, encoded_tx2],
            fee_token: None,
            signer_key: Some(fee_payer.to_string()),
            sig_verify: false,
            sign_only_indices: None,
        };

        let result = estimate_bundle_fee(&rpc_client, request).await.unwrap();

        // 16505 = base_fee(5000) + margin(10% of base_fee) + ATA rent + token-2022 transfer fee surcharge
        assert_eq!(result.fee_in_lamports, 16505);
    }
}
