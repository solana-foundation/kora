use crate::{
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP},
    metrics::run_metrics_server_if_required,
    rpc_server::{
        auth::{ApiKeyAuthLayer, HmacAuthLayer},
        rpc::KoraRpc,
    },
    state::get_config,
};
use http::{header, Method};
use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{net::SocketAddr, time::Duration};
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;

pub struct ServerHandles {
    pub rpc_handle: ServerHandle,
    pub metrics_handle: Option<ServerHandle>,
}

// We'll always prioritize the environment variable over the config value
fn get_value_by_priority(env_var: &str, config_value: Option<String>) -> Option<String> {
    std::env::var(env_var).ok().or(config_value)
}

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandles, anyhow::Error> {
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

    let config = get_config()?;

    let (metrics_handle, metrics_layers) = run_metrics_server_if_required(port).await?;

    let middleware = tower::ServiceBuilder::new()
        // Add metrics handler first (before other layers) so it can intercept /metrics
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        .layer(RateLimitLayer::new(config.kora.rate_limit, Duration::from_secs(1)))
        // Add metrics handler layer for Prometheus metrics
        .option_layer(
            metrics_layers.as_ref().and_then(|layers| layers.metrics_handler_layer.clone()),
        )
        .layer(cors)
        // Add metrics collection layer
        .option_layer(metrics_layers.as_ref().and_then(|layers| layers.http_metrics_layer.clone()))
        // Add authentication layer for API key if configured
        .option_layer(
            (get_value_by_priority("KORA_API_KEY", config.kora.auth.api_key.clone()))
                .map(|key| ApiKeyAuthLayer::new(key.clone())),
        )
        // Add authentication layer for HMAC if configured
        .option_layer(
            (get_value_by_priority("KORA_HMAC_SECRET", config.kora.auth.hmac_secret.clone())).map(
                |secret| HmacAuthLayer::new(secret.clone(), config.kora.auth.max_timestamp_age),
            ),
        );

    // Configure and build the server with HTTP support
    let server = ServerBuilder::default()
        .set_middleware(middleware)
        .http_only() // Explicitly enable HTTP
        .build(addr)
        .await?;

    let rpc_module = build_rpc_module(rpc)?;

    // Start the RPC server
    let rpc_handle = server
        .start(rpc_module)
        .map_err(|e| anyhow::anyhow!("Failed to start RPC server: {}", e))?;

    Ok(ServerHandles { rpc_handle, metrics_handle })
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
    let enabled_methods = &get_config()?.kora.enabled_methods;

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

    // Dev-only method for hot config reloading
    #[cfg(feature = "tests")]
    {
        let _ =
            module.register_async_method("updateConfig", |rpc_params, _rpc_context| async move {
                use crate::{config::Config, rpc_server::method::update_config};
                let config: Config = rpc_params.parse()?;
                update_config::update_config(config).await.map_err(Into::into)
            });
        log::info!("✓ updateConfig method registered (dev mode)");
    }

    Ok(module)
}
