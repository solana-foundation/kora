use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::error::KoraError;
#[cfg(feature = "docs")]
use utoipa::{
    openapi::{RefOr, Schema},
    ToSchema,
};

use crate::rpc_server::method::{
    estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
    },
    get_blockhash::{get_blockhash, GetBlockhashResponse},
    get_config::{get_config, GetConfigResponse},
    get_payer_signer::{get_payer_signer, GetPayerSignerResponse},
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
    get_version::{get_version, GetVersionResponse},
    sign_and_send_transaction::{
        sign_and_send_transaction, SignAndSendTransactionRequest, SignAndSendTransactionResponse,
    },
    sign_transaction::{sign_transaction, SignTransactionRequest, SignTransactionResponse},
    transfer_transaction::{
        transfer_transaction, TransferTransactionRequest, TransferTransactionResponse,
    },
};

#[derive(Clone)]
pub struct KoraRpc {
    rpc_client: Arc<RpcClient>,
}
#[cfg(feature = "docs")]
pub struct OpenApiSpec {
    pub name: String,
    pub request: Option<RefOr<Schema>>,
    pub response: RefOr<Schema>,
}

impl KoraRpc {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }

    pub fn get_rpc_client(&self) -> &Arc<RpcClient> {
        &self.rpc_client
    }

    pub async fn liveness(&self) -> Result<(), KoraError> {
        info!("Liveness request received");
        let result = Ok(());
        info!("Liveness response: {result:?}");
        result
    }

    pub async fn estimate_transaction_fee(
        &self,
        request: EstimateTransactionFeeRequest,
    ) -> Result<EstimateTransactionFeeResponse, KoraError> {
        info!("Estimate transaction fee request: {request:?}");
        let result = estimate_transaction_fee(&self.rpc_client, request).await;
        info!("Estimate transaction fee response: {result:?}");
        result
    }

    pub async fn get_supported_tokens(&self) -> Result<GetSupportedTokensResponse, KoraError> {
        info!("Get supported tokens request received");
        let result = get_supported_tokens().await;
        info!("Get supported tokens response: {result:?}");
        result
    }

    pub async fn get_payer_signer(&self) -> Result<GetPayerSignerResponse, KoraError> {
        info!("Get payer signer request received");
        let result = get_payer_signer().await;
        info!("Get payer signer response: {result:?}");
        result
    }

    pub async fn sign_transaction(
        &self,
        request: SignTransactionRequest,
    ) -> Result<SignTransactionResponse, KoraError> {
        info!("Sign transaction request: {request:?}");
        let result = sign_transaction(&self.rpc_client, request).await;
        info!("Sign transaction response: {result:?}");
        result
    }

    pub async fn sign_and_send_transaction(
        &self,
        request: SignAndSendTransactionRequest,
    ) -> Result<SignAndSendTransactionResponse, KoraError> {
        info!("Sign and send transaction request: {request:?}");
        let result = sign_and_send_transaction(&self.rpc_client, request).await;
        info!("Sign and send transaction response: {result:?}");
        result
    }

    pub async fn transfer_transaction(
        &self,
        request: TransferTransactionRequest,
    ) -> Result<TransferTransactionResponse, KoraError> {
        info!("Transfer transaction request: {request:?}");
        let result = transfer_transaction(&self.rpc_client, request).await;
        info!("Transfer transaction response: {result:?}");
        result
    }

    pub async fn get_blockhash(&self) -> Result<GetBlockhashResponse, KoraError> {
        info!("Get blockhash request received");
        let result = get_blockhash(&self.rpc_client).await;
        info!("Get blockhash response: {result:?}");
        result
    }

    pub async fn get_config(&self) -> Result<GetConfigResponse, KoraError> {
        info!("Get config request received");
        let result = get_config().await;
        info!("Get config response: {result:?}");
        result
    }

    pub async fn get_version(&self) -> Result<GetVersionResponse, KoraError> {
        info!("Get version request received");
        let result = get_version().await;
        info!("Get version response: {result:?}");
        result
    }

    #[cfg(feature = "docs")]
    pub fn build_docs_spec() -> Vec<OpenApiSpec> {
        vec![
            OpenApiSpec {
                name: "estimateTransactionFee".to_string(),
                request: Some(EstimateTransactionFeeRequest::schema().1),
                response: EstimateTransactionFeeResponse::schema().1,
            },
            OpenApiSpec {
                name: "getBlockhash".to_string(),
                request: None,
                response: GetBlockhashResponse::schema().1,
            },
            OpenApiSpec {
                name: "getConfig".to_string(),
                request: None,
                response: GetConfigResponse::schema().1,
            },
            OpenApiSpec {
                name: "getSupportedTokens".to_string(),
                request: None,
                response: GetSupportedTokensResponse::schema().1,
            },
            OpenApiSpec {
                name: "getPayerSigner".to_string(),
                request: None,
                response: GetPayerSignerResponse::schema().1,
            },
            OpenApiSpec {
                name: "signTransaction".to_string(),
                request: Some(SignTransactionRequest::schema().1),
                response: SignTransactionResponse::schema().1,
            },
            OpenApiSpec {
                name: "signAndSendTransaction".to_string(),
                request: Some(SignAndSendTransactionRequest::schema().1),
                response: SignAndSendTransactionResponse::schema().1,
            },
            OpenApiSpec {
                name: "transferTransaction".to_string(),
                request: Some(TransferTransactionRequest::schema().1),
                response: TransferTransactionResponse::schema().1,
            },
            OpenApiSpec {
                name: "getVersion".to_string(),
                request: None,
                response: GetVersionResponse::schema().1,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::update_config,
        tests::{
            common::setup_or_get_test_signer, config_mock::ConfigMockBuilder,
            rpc_mock::RpcMockBuilder,
        },
    };

    fn create_test_kora_rpc() -> KoraRpc {
        let rpc_client = RpcMockBuilder::new().build();
        KoraRpc::new(rpc_client)
    }

    #[tokio::test]
    async fn test_liveness() {
        let kora_rpc = create_test_kora_rpc();

        // Test liveness endpoint
        let result = kora_rpc.liveness().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_version() {
        let kora_rpc = create_test_kora_rpc();

        let result = kora_rpc.get_version().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.version.is_empty());
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_method_delegation_with_mocks() {
        // Setup test environment with both config and signer
        let config = ConfigMockBuilder::new().build();
        update_config(config).expect("Failed to update config");
        let _ = setup_or_get_test_signer(); // This initializes the signer pool

        let kora_rpc = create_test_kora_rpc();

        // Test liveness - should always succeed
        let liveness_result = kora_rpc.liveness().await;
        assert!(liveness_result.is_ok(), "Liveness should always succeed");

        // Test get_config - should work with mock config and signer pool
        let config_result = kora_rpc.get_config().await;
        assert!(config_result.is_ok(), "Get config failed: {:?}", config_result.err());

        // Test get_supported_tokens - should work with mock config
        let tokens_result = kora_rpc.get_supported_tokens().await;
        assert!(tokens_result.is_ok(), "Get supported tokens failed: {:?}", tokens_result.err());

        // Test get_payer_signer - should work with mock signer pool
        let signer_result = kora_rpc.get_payer_signer().await;
        assert!(signer_result.is_ok(), "Get payer signer failed: {:?}", signer_result.err());
    }
}
