use nonblocking::rpc_client::RpcClient;
use serde::Serialize;
use solana_client::nonblocking;
use solana_sdk::commitment_config::CommitmentConfig;

use kora_lib::error::KoraError;

#[derive(Debug, Serialize)]
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
