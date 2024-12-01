use std::sync::Arc;

use super::method::{
    estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
    },
    get_enabled_features::{get_enabled_features, GetEnabledFeaturesResponse},
    get_supported_tokens::{get_supported_tokens, GetSupportedTokensResponse},
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
        Self { rpc_client, features: config.features.enabled, tokens: config.tokens.enabled }
    }
    pub async fn liveness(&self) -> Result<(), KoraError> {
        Ok(())
    }

    pub async fn estimate_transaction_fee(
        &self,
        request: EstimateTransactionFeeRequest,
    ) -> Result<EstimateTransactionFeeResponse, KoraError> {
        estimate_transaction_fee(&self.rpc_client, request).await
    }

    pub async fn get_enabled_features(&self) -> Result<GetEnabledFeaturesResponse, KoraError> {
        get_enabled_features(&self.features).await
    }

    pub async fn get_supported_tokens(&self) -> Result<GetSupportedTokensResponse, KoraError> {
        get_supported_tokens(&self.tokens).await
    }
}
