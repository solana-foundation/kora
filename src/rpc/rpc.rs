use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use super::{error::KoraError, response::KoraResponse};

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
