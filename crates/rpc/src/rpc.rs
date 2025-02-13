use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use kora_lib::{
    config::{KoraConfig, ValidationConfig},
    error::KoraError,
};

use crate::method::{
    estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
    },
    get_blockhash::{get_blockhash, GetBlockhashResponse},
    get_config::{get_config, GetConfigResponse},
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
    sign_and_send_transaction::{
        sign_and_send_transaction, SignAndSendTransactionRequest, SignAndSendTransactionResult,
    },
    sign_transaction::{sign_transaction, SignTransactionRequest, SignTransactionResult},
    sign_transaction_if_paid::{
        sign_transaction_if_paid, SignTransactionIfPaidRequest, SignTransactionIfPaidResponse,
    },
    transfer_transaction::{
        transfer_transaction, TransferTransactionRequest, TransferTransactionResponse as TransferTransactionResponseOld,
    },
    transfer_transaction_v2::{transfer_transaction_v2, TransferTransactionV2Request, TransferTransactionResponse as TransferTransactionResponseV2},
};

#[derive(Clone)]
pub struct KoraRpc {
    rpc_client: Arc<RpcClient>,
    validation: ValidationConfig,
    pub config: KoraConfig,
}

impl KoraRpc {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        validation: ValidationConfig,
        config: KoraConfig,
    ) -> Self {
        Self { rpc_client, validation, config }
    }

    pub async fn liveness(&self) -> Result<(), KoraError> {
        info!("Liveness request received");
        let result = Ok(());
        info!("Liveness response: {:?}", result);
        result
    }

    pub async fn estimate_transaction_fee(
        &self,
        request: EstimateTransactionFeeRequest,
    ) -> Result<EstimateTransactionFeeResponse, KoraError> {
        info!("Estimate transaction fee request: {:?}", request);
        let result = estimate_transaction_fee(&self.rpc_client, request).await;
        info!("Estimate transaction fee response: {:?}", result);
        result
    }

    pub async fn get_supported_tokens(&self) -> Result<GetSupportedTokensResponse, KoraError> {
        info!("Get supported tokens request received");
        let result = get_supported_tokens(&self.validation.allowed_tokens).await;
        info!("Get supported tokens response: {:?}", result);
        result
    }

    pub async fn sign_transaction(
        &self,
        request: SignTransactionRequest,
    ) -> Result<SignTransactionResult, KoraError> {
        info!("Sign transaction request: {:?}", request);
        let result = sign_transaction(&self.rpc_client, &self.validation, request).await;
        info!("Sign transaction response: {:?}", result);
        result
    }

    pub async fn sign_and_send_transaction(
        &self,
        request: SignAndSendTransactionRequest,
    ) -> Result<SignAndSendTransactionResult, KoraError> {
        info!("Sign and send transaction request: {:?}", request);
        let result = sign_and_send_transaction(&self.rpc_client, &self.validation, request).await;
        info!("Sign and send transaction response: {:?}", result);
        result
    }

    pub async fn transfer_transaction(
        &self,
        request: TransferTransactionRequest,
    ) -> Result<TransferTransactionResponseOld, KoraError> {
        info!("Transfer transaction request: {:?}", request);
        let result = transfer_transaction(&self.rpc_client, &self.validation, request).await;
        info!("Transfer transaction response: {:?}", result);
        result
    }

    pub async fn get_blockhash(&self) -> Result<GetBlockhashResponse, KoraError> {
        info!("Get blockhash request received");
        let result = get_blockhash(&self.rpc_client).await;
        info!("Get blockhash response: {:?}", result);
        result
    }

    pub async fn get_config(&self) -> Result<GetConfigResponse, KoraError> {
        info!("Get config request received");
        let result = get_config(&self.validation).await;
        info!("Get config response: {:?}", result);
        result
    }

    pub async fn sign_transaction_if_paid(
        &self,
        request: SignTransactionIfPaidRequest,
    ) -> Result<SignTransactionIfPaidResponse, KoraError> {
        info!("Sign transaction if paid request: {:?}", request);
        let result = sign_transaction_if_paid(&self.rpc_client, &self.validation, request).await;
        info!("Sign transaction if paid response: {:?}", result);
        result
    }

    pub async fn transfer_transaction_v2(
        &self,
        request: TransferTransactionV2Request,
    ) -> Result<TransferTransactionResponseV2, KoraError> {
        info!("Transfer transaction V2 request: {:?}", request);
        let result = transfer_transaction_v2(&self.rpc_client, &self.validation, request).await;
        info!("Transfer transaction V2 response: {:?}", result);
        result
    }
}
