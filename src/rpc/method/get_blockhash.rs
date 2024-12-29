use nonblocking::rpc_client::RpcClient;
use serde::Serialize;
use solana_client::nonblocking;

use crate::common::KoraError;

#[derive(Debug, Serialize)]
pub struct GetBlockhashResponse {
    pub blockhash: String,
}

pub async fn get_blockhash(rpc_client: &RpcClient) -> Result<GetBlockhashResponse, KoraError> {
    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| KoraError::Rpc(format!("Failed to get blockhash: {}", e)))?;
    Ok(GetBlockhashResponse {
        blockhash: blockhash.to_string(),
    })
}
