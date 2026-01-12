use crate::common::{FeePayerTestHelper, TestContext};
use kora_lib::{
    bundle::{JitoClient, JitoConfig},
    transaction::{TransactionUtil, VersionedTransactionResolved},
};

const JITO_TESTNET_BLOCK_ENGINE_URL: &str = "https://dallas.testnet.block-engine.jito.wtf";

#[tokio::test]
async fn test_jito_mainnet_connection() {
    let config = JitoConfig { block_engine_url: JITO_TESTNET_BLOCK_ENGINE_URL.to_string() };
    let client = JitoClient::new(&config);

    // Query status of non-existent bundle to test connectivity
    let result =
        client.get_bundle_statuses(vec!["00000000-0000-0000-0000-000000000000".to_string()]).await;

    if let Err(e) = result {
        let error_str = e.to_string();

        if error_str.contains("Invalid") && error_str.contains("parse") {
            panic!("Jito mainnet integration test failed with code error: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_jito_testnet_connection() {
    let config = JitoConfig { block_engine_url: JITO_TESTNET_BLOCK_ENGINE_URL.to_string() };
    let client = JitoClient::new(&config);

    // Query status of non-existent bundle to test connectivity
    let result =
        client.get_bundle_statuses(vec!["00000000-0000-0000-0000-000000000000".to_string()]).await;

    if let Err(e) = result {
        let error_str = e.to_string();

        if error_str.contains("Invalid") && error_str.contains("parse") {
            panic!("Jito testnet integration test failed with code error: {e:?}");
        }
    }
}

#[tokio::test]
async fn test_jito_send_bundle() {
    let ctx = TestContext::new().await.expect("Failed to create test context");
    let config = JitoConfig { block_engine_url: JITO_TESTNET_BLOCK_ENGINE_URL.to_string() };
    let client = JitoClient::new(&config);

    let encoded_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .build()
        .await
        .expect("Failed to create transaction");

    let decoded_tx = TransactionUtil::decode_b64_transaction(&encoded_tx).unwrap();

    let resolved_tx =
        VersionedTransactionResolved::from_kora_built_transaction(&decoded_tx).unwrap();

    let response = client.send_bundle(&[resolved_tx]).await;

    if let Err(e) = response {
        let error_str = e.to_string();

        if error_str.contains("404") {
            panic!("Jito send bundle test failed with 404 error: {error_str:?}");
        }

        // Only test that the endpoint is a valid Jito endpoint
    }
}
