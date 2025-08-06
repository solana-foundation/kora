use crate::{
    auth::{ApiKeyAuthLayer, HmacAuthLayer},
    rpc::KoraRpc,
};
use http::{header, Method};
use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use kora_lib::constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP};
use std::{net::SocketAddr, time::Duration};
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;

// We'll always prioritize the environment variable over the config value
fn get_value_by_priority(env_var: &str, config_value: Option<String>) -> Option<String> {
    std::env::var(env_var).ok().or(config_value)
}

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandle, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {addr}, port {port}");

    // Build middleware stack with tracing and CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static(X_API_KEY),
            header::HeaderName::from_static(X_HMAC_SIGNATURE),
            header::HeaderName::from_static(X_TIMESTAMP),
        ])
        .max_age(Duration::from_secs(3600));

    let middleware = tower::ServiceBuilder::new()
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        .layer(RateLimitLayer::new(rpc.config.rate_limit, Duration::from_secs(1)))
        .layer(cors)
        // Add authentication layer for API key if configured
        .option_layer(
            (get_value_by_priority("KORA_API_KEY", rpc.config.api_key.clone()))
                .map(|key| ApiKeyAuthLayer::new(key.clone())),
        )
        // Add authentication layer for HMAC if configured
        .option_layer(
            (get_value_by_priority("KORA_HMAC_SECRET", rpc.config.hmac_secret.clone()))
                .map(|secret| HmacAuthLayer::new(secret.clone(), rpc.config.max_timestamp_age)),
        );

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

macro_rules! register_method_if_enabled {
    // For methods without parameters
    ($module:expr, $enabled_methods:expr, $field:ident, $method_name:expr, $rpc_method:ident) => {
        if $enabled_methods.$field {
            let _ = $module.register_async_method(
                $method_name,
                |_rpc_params, rpc_context| async move {
                    let rpc = rpc_context.as_ref();
                    rpc.$rpc_method().await.map_err(Into::into)
                },
            );
        }
    };

    // For methods with parameters
    ($module:expr, $enabled_methods:expr, $field:ident, $method_name:expr, $rpc_method:ident, with_params) => {
        if $enabled_methods.$field {
            let _ =
                $module.register_async_method($method_name, |rpc_params, rpc_context| async move {
                    let rpc = rpc_context.as_ref();
                    let params = rpc_params.parse()?;
                    rpc.$rpc_method(params).await.map_err(Into::into)
                });
        }
    };
}

fn build_rpc_module(rpc: KoraRpc) -> Result<RpcModule<KoraRpc>, anyhow::Error> {
    let mut module = RpcModule::new(rpc.clone());
    let enabled_methods = &rpc.config.enabled_methods;

    register_method_if_enabled!(module, enabled_methods, liveness, "liveness", liveness);

    register_method_if_enabled!(
        module,
        enabled_methods,
        estimate_transaction_fee,
        "estimateTransactionFee",
        estimate_transaction_fee,
        with_params
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        get_supported_tokens,
        "getSupportedTokens",
        get_supported_tokens
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        sign_transaction,
        "signTransaction",
        sign_transaction,
        with_params
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        sign_and_send_transaction,
        "signAndSendTransaction",
        sign_and_send_transaction,
        with_params
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        transfer_transaction,
        "transferTransaction",
        transfer_transaction,
        with_params
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        get_blockhash,
        "getBlockhash",
        get_blockhash
    );
    register_method_if_enabled!(module, enabled_methods, get_config, "getConfig", get_config);
    register_method_if_enabled!(
        module,
        enabled_methods,
        sign_transaction_if_paid,
        "signTransactionIfPaid",
        sign_transaction_if_paid,
        with_params
    );

    Ok(module)
}
