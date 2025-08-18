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
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
    sign_and_send_transaction::{
        sign_and_send_transaction, SignAndSendTransactionRequest, SignAndSendTransactionResponse,
    },
    sign_transaction::{sign_transaction, SignTransactionRequest, SignTransactionResponse},
    sign_transaction_if_paid::{
        sign_transaction_if_paid, SignTransactionIfPaidRequest, SignTransactionIfPaidResponse,
    },
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

    pub async fn sign_transaction_if_paid(
        &self,
        request: SignTransactionIfPaidRequest,
    ) -> Result<SignTransactionIfPaidResponse, KoraError> {
        info!("Sign transaction if paid request: {request:?}");
        let result = sign_transaction_if_paid(&self.rpc_client, request).await;
        info!("Sign transaction if paid response: {result:?}");
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
                name: "signTransactionIfPaid".to_string(),
                request: Some(SignTransactionIfPaidRequest::schema().1),
                response: SignTransactionIfPaidResponse::schema().1,
            },
        ]
    }
}
