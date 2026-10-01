use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::error::KoraError;
#[cfg(feature = "docs")]
use utoipa::{
    openapi::{RefOr, Schema},
    PartialSchema,
};

#[allow(deprecated)]
use crate::rpc_server::method::{
    estimate_bundle_fee::{
        estimate_bundle_fee, EstimateBundleFeeRequest, EstimateBundleFeeResponse,
    },
    estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
    },
    get_blockhash::{get_blockhash, GetBlockhashResponse},
    get_config::{get_config, GetConfigResponse},
    get_payer_signer::{get_payer_signer, GetPayerSignerResponse},
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
    get_version::{get_version, GetVersionResponse},
    sign_and_send_bundle::{
        sign_and_send_bundle, SignAndSendBundleRequest, SignAndSendBundleResponse,
    },
    sign_and_send_transaction::{
        sign_and_send_transaction, SignAndSendTransactionRequest, SignAndSendTransactionResponse,
    },
    sign_bundle::{sign_bundle, SignBundleRequest, SignBundleResponse},
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

fn log_response<T: std::fmt::Debug>(
    label: &str,
    result: Result<T, KoraError>,
) -> Result<T, KoraError> {
    info!("{label} response: {result:?}");
    result
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
        log_response("Liveness", Ok(()))
    }

    pub async fn estimate_transaction_fee(
        &self,
        request: EstimateTransactionFeeRequest,
    ) -> Result<EstimateTransactionFeeResponse, KoraError> {
        info!("Estimate transaction fee request: {request:?}");
        log_response(
            "Estimate transaction fee",
            estimate_transaction_fee(&self.rpc_client, request).await,
        )
    }

    pub async fn estimate_bundle_fee(
        &self,
        request: EstimateBundleFeeRequest,
    ) -> Result<EstimateBundleFeeResponse, KoraError> {
        info!("Estimate bundle fee request: {request:?}");
        log_response("Estimate bundle fee", estimate_bundle_fee(&self.rpc_client, request).await)
    }

    pub async fn get_supported_tokens(&self) -> Result<GetSupportedTokensResponse, KoraError> {
        info!("Get supported tokens request received");
        log_response("Get supported tokens", get_supported_tokens().await)
    }

    pub async fn get_payer_signer(&self) -> Result<GetPayerSignerResponse, KoraError> {
        info!("Get payer signer request received");
        log_response("Get payer signer", get_payer_signer().await)
    }

    pub async fn sign_transaction(
        &self,
        request: SignTransactionRequest,
    ) -> Result<SignTransactionResponse, KoraError> {
        info!("Sign transaction request: {request:?}");
        log_response("Sign transaction", sign_transaction(&self.rpc_client, request).await)
    }

    pub async fn sign_and_send_transaction(
        &self,
        request: SignAndSendTransactionRequest,
    ) -> Result<SignAndSendTransactionResponse, KoraError> {
        info!("Sign and send transaction request: {request:?}");
        log_response(
            "Sign and send transaction",
            sign_and_send_transaction(&self.rpc_client, request).await,
        )
    }

    #[deprecated(since = "2.2.0", note = "Use getPaymentInstruction instead for fee payment flows")]
    pub async fn transfer_transaction(
        &self,
        request: TransferTransactionRequest,
    ) -> Result<TransferTransactionResponse, KoraError> {
        info!("Transfer transaction request: {request:?}");
        #[allow(deprecated)]
        let result = transfer_transaction(&self.rpc_client, request).await;
        log_response("Transfer transaction", result)
    }

    pub async fn get_blockhash(&self) -> Result<GetBlockhashResponse, KoraError> {
        info!("Get blockhash request received");
        log_response("Get blockhash", get_blockhash(&self.rpc_client).await)
    }

    pub async fn get_config(&self) -> Result<GetConfigResponse, KoraError> {
        info!("Get config request received");
        log_response("Get config", get_config().await)
    }

    pub async fn get_version(&self) -> Result<GetVersionResponse, KoraError> {
        info!("Get version request received");
        log_response("Get version", get_version().await)
    }

    pub async fn sign_bundle(
        &self,
        request: SignBundleRequest,
    ) -> Result<SignBundleResponse, KoraError> {
        info!("Sign bundle request: {request:?}");
        log_response("Sign bundle", sign_bundle(&self.rpc_client, request).await)
    }

    pub async fn sign_and_send_bundle(
        &self,
        request: SignAndSendBundleRequest,
    ) -> Result<SignAndSendBundleResponse, KoraError> {
        info!("Sign and send bundle request: {request:?}");
        log_response("Sign and send bundle", sign_and_send_bundle(&self.rpc_client, request).await)
    }

    #[cfg(feature = "docs")]
    pub fn build_docs_spec() -> Vec<OpenApiSpec> {
        vec![
            OpenApiSpec {
                name: "estimateTransactionFee".to_string(),
                request: Some(EstimateTransactionFeeRequest::schema()),
                response: EstimateTransactionFeeResponse::schema(),
            },
            OpenApiSpec {
                name: "estimateBundleFee".to_string(),
                request: Some(EstimateBundleFeeRequest::schema()),
                response: EstimateBundleFeeResponse::schema(),
            },
            OpenApiSpec {
                name: "getBlockhash".to_string(),
                request: None,
                response: GetBlockhashResponse::schema(),
            },
            OpenApiSpec {
                name: "getConfig".to_string(),
                request: None,
                response: GetConfigResponse::schema(),
            },
            OpenApiSpec {
                name: "getSupportedTokens".to_string(),
                request: None,
                response: GetSupportedTokensResponse::schema(),
            },
            OpenApiSpec {
                name: "getPayerSigner".to_string(),
                request: None,
                response: GetPayerSignerResponse::schema(),
            },
            OpenApiSpec {
                name: "signTransaction".to_string(),
                request: Some(SignTransactionRequest::schema()),
                response: SignTransactionResponse::schema(),
            },
            OpenApiSpec {
                name: "signAndSendTransaction".to_string(),
                request: Some(SignAndSendTransactionRequest::schema()),
                response: SignAndSendTransactionResponse::schema(),
            },
            OpenApiSpec {
                name: "transferTransaction".to_string(),
                request: Some(TransferTransactionRequest::schema()),
                response: TransferTransactionResponse::schema(),
            },
            OpenApiSpec {
                name: "getVersion".to_string(),
                request: None,
                response: GetVersionResponse::schema(),
            },
            OpenApiSpec {
                name: "signBundle".to_string(),
                request: Some(SignBundleRequest::schema()),
                response: SignBundleResponse::schema(),
            },
            OpenApiSpec {
                name: "signAndSendBundle".to_string(),
                request: Some(SignAndSendBundleRequest::schema()),
                response: SignAndSendBundleResponse::schema(),
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
        let config = ConfigMockBuilder::new().build();
        update_config(config).expect("Failed to update config");
        let _ = setup_or_get_test_signer();

        let kora_rpc = create_test_kora_rpc();

        let liveness_result = kora_rpc.liveness().await;
        assert!(liveness_result.is_ok(), "Liveness should always succeed");

        let config_result = kora_rpc.get_config().await;
        assert!(config_result.is_ok(), "Get config failed: {:?}", config_result.err());

        let tokens_result = kora_rpc.get_supported_tokens().await;
        assert!(tokens_result.is_ok(), "Get supported tokens failed: {:?}", tokens_result.err());

        let signer_result = kora_rpc.get_payer_signer().await;
        assert!(signer_result.is_ok(), "Get payer signer failed: {:?}", signer_result.err());
    }
}
