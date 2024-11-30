use std::sync::Arc;

use super::{error::KoraError, response::KoraResponse};
use solana_client::nonblocking::rpc_client::RpcClient;

pub struct KoraRpc {
    rpc_client: Arc<RpcClient>,
}

impl KoraRpc {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }
    pub async fn liveness(&self) -> Result<KoraResponse<()>, KoraError> {
        Ok(KoraResponse::ok(()))
    }
}
