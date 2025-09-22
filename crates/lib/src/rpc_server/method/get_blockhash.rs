use crate::error::KoraError;
use nonblocking::rpc_client::RpcClient;
use serde::Serialize;
use solana_client::nonblocking;
use solana_commitment_config::CommitmentConfig;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct GetBlockhashResponse {
    pub blockhash: String,
}

pub async fn get_blockhash(rpc_client: &RpcClient) -> Result<GetBlockhashResponse, KoraError> {
    let blockhash = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;
    Ok(GetBlockhashResponse { blockhash: blockhash.0.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{config_mock::ConfigMockBuilder, rpc_mock::RpcMockBuilder};

    #[tokio::test]
    async fn test_get_blockhash_success() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let rpc_client = RpcMockBuilder::new().with_blockhash().build();

        let result = get_blockhash(&rpc_client).await;

        assert!(result.is_ok(), "Should successfully get blockhash");
        let response = result.unwrap();
        assert!(!response.blockhash.is_empty(), "Blockhash should not be empty");
    }
}
