use std::time::Duration;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

pub fn get_rpc_client(rpc_url: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(90),
        CommitmentConfig::confirmed(),
    ))
}