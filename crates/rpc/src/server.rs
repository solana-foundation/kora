use crate::rpc::KoraRpc;
use http::{header, Method};
use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{net::SocketAddr, time::Duration};
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;
use crate::actions::transfer::{get_transfer_metadata, handle_transfer_action, TransferActionRequest};
use warp;
use warp::Rejection;

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandle, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {}, port {}", addr, port);

    // Build middleware stack with tracing and CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::CONTENT_ENCODING,
            header::ACCEPT_ENCODING
        ])
        .max_age(Duration::from_secs(3600));

    let middleware = tower::ServiceBuilder::new()
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        .layer(RateLimitLayer::new(rpc.config.rate_limit, Duration::from_secs(1)))
        .layer(cors);

    // Create transfer action routes
    let transfer_route = warp::path!("api" / "v1" / "actions" / "transfer")
        .and(warp::get())
        .and_then(|| async {
            let metadata = get_transfer_metadata().await;
            Ok::<_, Rejection>(warp::reply::json(&metadata))
        });

    let transfer_action_route = warp::path!("api" / "v1" / "actions" / "transfer")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rpc(rpc.clone()))
        .and_then(|request: TransferActionRequest, rpc: KoraRpc| async move {
            let response = handle_transfer_action(
                &rpc.rpc_client,
                &rpc.validation,
                request
            ).await?;
            Ok::<_, Rejection>(warp::reply::json(&response))
        });

    let options_route = warp::path!("api" / "v1" / "actions" / "transfer")
        .and(warp::options())
        .map(|| {
            warp::reply::with_header(
                warp::reply::reply(),
                "Access-Control-Allow-Origin",
                "*"
            )
        });

    // Combine all routes
    let routes = transfer_route
        .or(transfer_action_route)
        .or(options_route)
        .with(cors);

    // Configure and build the server with HTTP support
    let server = ServerBuilder::default()
        .set_middleware(middleware)
        .http_only()
        .build(addr)
        .await?;

    let rpc_module = build_rpc_module(rpc)?;

    // Start both the RPC server and warp server
    let rpc_handle = server.start(rpc_module)?;
    
    // Start warp server
    tokio::spawn(warp::serve(routes).run(addr));

    Ok(rpc_handle)
}

// Helper function to pass RPC context to handlers
fn with_rpc(rpc: KoraRpc) -> impl Filter<Extract = (KoraRpc,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || rpc.clone())
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

    let _ = module.register_async_method("getTransferMetadata", |_params, rpc_context| async move {
        let _rpc = rpc_context.as_ref();
        get_transfer_metadata().await
    });

    let _ = module.register_async_method("transferAction", |rpc_params, rpc_context| async move {
        let rpc = rpc_context.as_ref();
        let request = rpc_params.parse()?;
        handle_transfer_action(&rpc.rpc_client, &rpc.validation, request).await.map_err(Into::into)
    });

    Ok(module)
}
