use crate::rpc::KoraRpc;
use http::{header, Method};
use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{net::SocketAddr, time::Duration};
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandle, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {addr}, port {port}");

    // Build middleware stack with tracing and CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(Duration::from_secs(3600));

    let middleware = tower::ServiceBuilder::new()
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        .layer(RateLimitLayer::new(rpc.config.rate_limit, Duration::from_secs(1)))
        .layer(cors);

    // Configure and build the server with HTTP support
    let server = ServerBuilder::default()
        .set_middleware(middleware)
        .http_only() // Explicitly enable HTTP
        .build(addr)
        .await?;

    let rpc_module = build_rpc_module(rpc)?;

    // Start the server and return the handle
    server.start(rpc_module).map_err(|e| anyhow::anyhow!("Failed to start RPC server: {}", e))
}

fn build_rpc_module(rpc: KoraRpc) -> Result<RpcModule<KoraRpc>, anyhow::Error> {
    let mut module = RpcModule::new(rpc);

    let _ = module.register_async_method("liveness", |_params, _rpc_context| async move {
        log::debug!("liveness called");
        let rpc = _rpc_context.as_ref();
        rpc.liveness().await.map_err(Into::into)
    });

    let _ = module.register_async_method(
        "estimateTransactionFee",
        |rpc_params, rpc_context| async move {
            let rpc = rpc_context.as_ref();
            let request = rpc_params.parse()?;
            rpc.estimate_transaction_fee(request).await.map_err(Into::into)
        },
    );

    let _ =
        module.register_async_method("getSupportedTokens", |_rpc_params, rpc_context| async move {
            let rpc = rpc_context.as_ref();
            rpc.get_supported_tokens().await.map_err(Into::into)
        });

    let _ = module.register_async_method("signTransaction", |rpc_params, rpc_context| async move {
        let rpc = rpc_context.as_ref();
        let params = rpc_params.parse()?;
        rpc.sign_transaction(params).await.map_err(Into::into)
    });

    let _ = module.register_async_method(
        "signAndSendTransaction",
        |rpc_params, rpc_context| async move {
            let rpc = rpc_context.as_ref();
            let params = rpc_params.parse()?;
            rpc.sign_and_send_transaction(params).await.map_err(Into::into)
        },
    );

    let _ =
        module.register_async_method("transferTransaction", |rpc_params, rpc_context| async move {
            let rpc = rpc_context.as_ref();
            let params = rpc_params.parse()?;
            rpc.transfer_transaction(params).await.map_err(Into::into)
        });

    let _ = module.register_async_method("getBlockhash", |_rpc_params, rpc_context| async move {
        let rpc = rpc_context.as_ref();
        rpc.get_blockhash().await.map_err(Into::into)
    });

    let _ = module.register_async_method("getConfig", |_rpc_params, rpc_context| async move {
        let rpc = rpc_context.as_ref();
        rpc.get_config().await.map_err(Into::into)
    });

    let _ = module.register_async_method(
        "signTransactionIfPaid",
        |rpc_params, rpc_context| async move {
            let rpc = rpc_context.as_ref();
            let params = rpc_params.parse()?;
            rpc.sign_transaction_if_paid(params).await.map_err(Into::into)
        },
    );

    Ok(module)
}
