use log::info;
use std::sync::Arc;

use super::method::{
    estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
    },
    get_enabled_features::{get_enabled_features, GetEnabledFeaturesResponse},
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
    sign_and_send::{sign_and_send, SignAndSendTransactionRequest, SignAndSendTransactionResult},
    sign_transaction::{sign_transaction, SignTransactionRequest, SignTransactionResult},
};
use crate::common::{error::KoraError, Config, Feature};
use solana_client::nonblocking::rpc_client::RpcClient;

pub struct KoraRpc {
    rpc_client: Arc<RpcClient>,
    features: Vec<Feature>,
    tokens: Vec<String>,
}

impl KoraRpc {
    pub fn new(rpc_client: Arc<RpcClient>, config: Config) -> Self {
        Self { rpc_client, features: config.features.enabled, tokens: config.tokens.allowed }
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

    pub async fn get_enabled_features(&self) -> Result<GetEnabledFeaturesResponse, KoraError> {
        info!("Get enabled features request received");
        let result = get_enabled_features(&self.features).await;
        info!("Get enabled features response: {:?}", result);
        result
    }

    pub async fn get_supported_tokens(&self) -> Result<GetSupportedTokensResponse, KoraError> {
        info!("Get supported tokens request received");
        let result = get_supported_tokens(&self.tokens).await;
        info!("Get supported tokens response: {:?}", result);
        result
    }

    pub async fn sign_transaction(
        &self,
        request: SignTransactionRequest,
    ) -> Result<SignTransactionResult, KoraError> {
        info!("Sign transaction request: {:?}", request);
        let result = sign_transaction(&self.rpc_client, request)
            .await
            .map_err(|e| KoraError::SigningError(e.to_string()));
        info!("Sign transaction response: {:?}", result);
        result
    }

    pub async fn sign_and_send(
        &self,
        request: SignAndSendTransactionRequest,
    ) -> Result<SignAndSendTransactionResult, KoraError> {
        info!("Sign and send request: {:?}", request);
        let result = sign_and_send(&self.rpc_client, request).await;
        info!("Sign and send response: {:?}", result);
        result
    }
}
