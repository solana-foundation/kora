use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use std::net::SocketAddr;

use super::rpc::KoraRpc;

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandle, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {}, port {}", addr, port);
    let middleware =
        tower::ServiceBuilder::new().layer(ProxyGetRequestLayer::new("/liveness", "liveness")?);

    let server = ServerBuilder::default().set_middleware(middleware).build(addr).await?;
    let rpc_module = build_rpc_module(rpc)?;
    server.start(rpc_module).map_err(|e| anyhow::anyhow!("Failed to start RPC server: {}", e))
}

fn build_rpc_module(rpc: KoraRpc) -> Result<RpcModule<KoraRpc>, anyhow::Error> {
    let mut module = RpcModule::new(rpc);

    module.register_async_method("liveness", |_params, _rpc_context| async move {
        log::debug!("liveness called");
        let rpc = _rpc_context.as_ref();
        rpc.liveness().await.map_err(Into::into)
    })?;
    Ok(module)
}
