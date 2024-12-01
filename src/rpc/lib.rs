use std::sync::Arc;

use super::method::estimate_transaction_fee::{
    estimate_transaction_fee, EstimateTransactionFeeRequest, EstimateTransactionFeeResponse,
};
use crate::common::error::KoraError;
use solana_client::nonblocking::rpc_client::RpcClient;

pub struct KoraRpc {
    rpc_client: Arc<RpcClient>,
}

impl KoraRpc {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
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
}
