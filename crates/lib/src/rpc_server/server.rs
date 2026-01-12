use crate::{
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP},
    metrics::run_metrics_server_if_required,
    rpc_server::{
        auth::{ApiKeyAuthLayer, HmacAuthLayer},
        middleware_utils::MethodValidationLayer,
        rpc::KoraRpc,
    },
    usage_limit::UsageTracker,
};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;
use http::{header, Method};
use jsonrpsee::{
    server::{middleware::proxy_get_request::ProxyGetRequestLayer, ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{net::SocketAddr, time::Duration};
use tokio::task::JoinHandle;
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;

pub struct ServerHandles {
    pub rpc_handle: ServerHandle,
    pub metrics_handle: Option<ServerHandle>,
    pub balance_tracker_handle: Option<JoinHandle<()>>,
}

// We'll always prioritize the environment variable over the config value
fn get_value_by_priority(env_var: &str, config_value: Option<String>) -> Option<String> {
    std::env::var(env_var).ok().or(config_value)
}

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandles, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {addr}, port {port}");

    // Initialize usage limiter
    if let Err(e) = UsageTracker::init_usage_limiter().await {
        log::error!("Failed to initialize usage limiter: {e}");
        return Err(anyhow::anyhow!("Usage limiter initialization failed: {e}"));
    }

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

    // Get the RPC client from KoraRpc to pass to metrics initialization
    let rpc_client = rpc.get_rpc_client().clone();

    let (metrics_handle, metrics_layers, balance_tracker_handle) =
        run_metrics_server_if_required(port, rpc_client).await?;

    // Build whitelist of allowed methods from enabled_methods config
    let allowed_methods = config.kora.enabled_methods.get_enabled_method_names();

    let middleware = tower::ServiceBuilder::new()
        // Add metrics handler first (before other layers) so it can intercept /metrics
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        .layer(RateLimitLayer::new(config.kora.rate_limit, Duration::from_secs(1)))
        // Add metrics handler layer for Prometheus metrics
        .option_layer(
            metrics_layers.as_ref().and_then(|layers| layers.metrics_handler_layer.clone()),
        )
        .layer(cors)
        // Method validation layer -  to fail fast
        .layer(MethodValidationLayer::new(allowed_methods.clone()))
        // Add metrics collection layer
        .option_layer(metrics_layers.as_ref().and_then(|layers| layers.http_metrics_layer.clone()))
        // Add authentication layer for API key if configured
        .option_layer(
            (get_value_by_priority("KORA_API_KEY", config.kora.auth.api_key.clone()))
                .map(ApiKeyAuthLayer::new),
        )
        // Add authentication layer for HMAC if configured
        .option_layer(
            (get_value_by_priority("KORA_HMAC_SECRET", config.kora.auth.hmac_secret.clone()))
                .map(|secret| HmacAuthLayer::new(secret, config.kora.auth.max_timestamp_age)),
        );

    // Configure and build the server with HTTP support
    let server = ServerBuilder::default()
        .max_request_body_size(config.kora.max_request_body_size as u32)
        .set_middleware(middleware)
        .http_only() // Explicitly enable HTTP
        .build(addr)
        .await?;

    let rpc_module = build_rpc_module(rpc)?;

    // Start the RPC server
    let rpc_handle = server
        .start(rpc_module)
        .map_err(|e| anyhow::anyhow!("Failed to start RPC server: {}", e))?;

    Ok(ServerHandles { rpc_handle, metrics_handle, balance_tracker_handle })
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
            #[allow(deprecated)]
            let _ =
                $module.register_async_method($method_name, |rpc_params, rpc_context| async move {
                    let rpc = rpc_context.as_ref();
                    let params = rpc_params.parse()?;
                    #[allow(deprecated)]
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
        estimate_bundle_fee,
        "estimateBundleFee",
        estimate_bundle_fee,
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
        get_payer_signer,
        "getPayerSigner",
        get_payer_signer
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
    register_method_if_enabled!(module, enabled_methods, get_version, "getVersion", get_version);
    register_method_if_enabled!(
        module,
        enabled_methods,
        sign_bundle,
        "signBundle",
        sign_bundle,
        with_params
    );
    register_method_if_enabled!(
        module,
        enabled_methods,
        sign_and_send_bundle,
        "signAndSendBundle",
        sign_and_send_bundle,
        with_params
    );

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::EnabledMethods,
        tests::{
            common::setup_or_get_test_signer,
            config_mock::{ConfigMockBuilder, KoraConfigBuilder},
            rpc_mock::RpcMockBuilder,
        },
    };
    use std::env;

    #[test]
    fn test_get_value_by_priority_env_var_takes_precedence() {
        let env_var_name = "TEST_ENV_VAR_PRECEDENCE_UNIQUE";
        env::set_var(env_var_name, "env_value");

        let result = get_value_by_priority(env_var_name, Some("config_value".to_string()));
        assert_eq!(result, Some("env_value".to_string()));

        env::remove_var(env_var_name);
    }

    #[test]
    fn test_get_value_by_priority_config_fallback() {
        let env_var_name = "TEST_ENV_VAR_FALLBACK_UNIQUE_XYZ123";

        let result = get_value_by_priority(env_var_name, Some("config_value".to_string()));
        assert_eq!(result, Some("config_value".to_string()));
    }

    #[test]
    fn test_get_value_by_priority_none_when_both_missing() {
        let env_var_name = "TEST_ENV_VAR_MISSING_UNIQUE_ABC789";

        let result = get_value_by_priority(env_var_name, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_rpc_module_all_methods_enabled() {
        // Default is all methods enabled
        let enabled_methods = EnabledMethods::default();

        let kora_config = KoraConfigBuilder::new().with_enabled_methods(enabled_methods).build();
        let _m = ConfigMockBuilder::new().with_kora(kora_config).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = RpcMockBuilder::new().build();
        let kora_rpc = KoraRpc::new(rpc_client);

        let result = build_rpc_module(kora_rpc);
        assert!(result.is_ok(), "Failed to build RPC module with all methods enabled");

        // Verify that the module has the expected methods
        let module = result.unwrap();
        let method_names: Vec<&str> = module.method_names().collect();
        assert_eq!(method_names.len(), 10);
        assert!(method_names.contains(&"liveness"));
        assert!(method_names.contains(&"estimateTransactionFee"));
        assert!(method_names.contains(&"getSupportedTokens"));
        assert!(method_names.contains(&"getPayerSigner"));
        assert!(method_names.contains(&"signTransaction"));
        assert!(method_names.contains(&"signAndSendTransaction"));
        assert!(method_names.contains(&"transferTransaction"));
        assert!(method_names.contains(&"getBlockhash"));
        assert!(method_names.contains(&"getConfig"));
        assert!(method_names.contains(&"getVersion"));
        // Note: signBundle is NOT included by default (opt-in via enabled_methods.sign_bundle)
    }

    #[test]
    fn test_build_rpc_module_all_methods_disabled() {
        // Setup config with all methods disabled
        let enabled_methods = EnabledMethods {
            estimate_transaction_fee: false,
            get_supported_tokens: false,
            get_payer_signer: false,
            sign_transaction: false,
            sign_and_send_transaction: false,
            transfer_transaction: false,
            get_blockhash: false,
            get_config: false,
            get_version: false,
            liveness: false,
            estimate_bundle_fee: false,
            sign_and_send_bundle: false,
            sign_bundle: false,
        };

        let kora_config = KoraConfigBuilder::new().with_enabled_methods(enabled_methods).build();
        let _m = ConfigMockBuilder::new().with_kora(kora_config).build_and_setup();
        let _ = setup_or_get_test_signer();

        // Create RPC module
        let rpc_client = RpcMockBuilder::new().build();
        let kora_rpc = KoraRpc::new(rpc_client);

        // Build the module - should succeed even with no methods
        let result = build_rpc_module(kora_rpc);
        assert!(result.is_ok(), "Failed to build RPC module with all methods disabled");

        assert_eq!(result.unwrap().method_names().count(), 0);
    }

    #[test]
    fn test_build_rpc_module_selective_methods() {
        // Setup config with only some methods enabled
        let enabled_methods = EnabledMethods {
            liveness: true,
            get_config: true,
            get_supported_tokens: true,
            estimate_transaction_fee: false,
            get_payer_signer: false,
            sign_transaction: false,
            sign_and_send_transaction: false,
            transfer_transaction: false,
            get_blockhash: false,
            get_version: false,
            estimate_bundle_fee: false,
            sign_and_send_bundle: false,
            sign_bundle: false,
        };

        let kora_config = KoraConfigBuilder::new().with_enabled_methods(enabled_methods).build();
        let _m = ConfigMockBuilder::new().with_kora(kora_config).build_and_setup();
        let _ = setup_or_get_test_signer();

        // Create RPC module
        let rpc_client = RpcMockBuilder::new().build();
        let kora_rpc = KoraRpc::new(rpc_client);

        // Build the module
        let result = build_rpc_module(kora_rpc);
        assert!(result.is_ok(), "Failed to build RPC module with selective methods");

        // Verify that only the expected methods are registered
        let module = result.unwrap();
        let method_names: Vec<&str> = module.method_names().collect();
        assert_eq!(method_names.len(), 3);
        assert!(method_names.contains(&"liveness"));
        assert!(method_names.contains(&"getConfig"));
        assert!(method_names.contains(&"getSupportedTokens"));
    }
}
