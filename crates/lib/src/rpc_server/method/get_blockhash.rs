use crate::{error::KoraError, CacheUtil};
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use utoipa::ToSchema;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

#[derive(Debug, Serialize, ToSchema)]
pub struct GetBlockhashResponse {
    pub blockhash: String,
}

pub async fn get_blockhash(rpc_client: &RpcClient) -> Result<GetBlockhashResponse, KoraError> {
    let config = get_config()?;

    #[cfg(not(test))]
    let blockhash = CacheUtil::get_or_fetch_latest_blockhash(config, rpc_client).await?;
    #[cfg(test)]
    let blockhash = CacheUtil::get_or_fetch_latest_blockhash(&config, rpc_client).await?;

    Ok(GetBlockhashResponse { blockhash: blockhash.to_string() })
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
