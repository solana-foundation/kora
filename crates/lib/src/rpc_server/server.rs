use crate::{
    config::AuthConfig,
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_RECAPTCHA_TOKEN, X_TIMESTAMP},
    metrics::run_metrics_server_if_required,
    rpc_server::{
        auth::{ApiKeyAuthLayer, HmacAuthLayer},
        middleware_utils::MethodValidationLayer,
        rate_limit::IdentityRateLimitLayer,
        recaptcha::RecaptchaLayer,
        recaptcha_util::RecaptchaConfig,
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
use tower_http::cors::CorsLayer;

pub struct ServerHandles {
    pub rpc_handle: ServerHandle,
    pub metrics_handle: Option<ServerHandle>,
    pub balance_tracker_handle: Option<JoinHandle<()>>,
}

// We'll always prioritize the environment variable over the config value
fn get_value_by_priority(env_var: &str, config_value: Option<String>) -> Option<String> {
    AuthConfig::normalize_optional_secret(std::env::var(env_var).ok())
        .or_else(|| AuthConfig::normalize_optional_secret(config_value))
}

pub async fn run_rpc_server(rpc: KoraRpc, port: u16) -> Result<ServerHandles, anyhow::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("RPC server started on {addr}, port {port}");

    // Initialize usage limiter
    if let Err(e) = UsageTracker::init_usage_limiter().await {
        log::error!("Failed to initialize usage limiter: {e}");
        return Err(anyhow::anyhow!("Usage limiter initialization failed: {e}"));
    }

    let config = get_config()?;

    let allow_origins = if config.kora.cors_allow_origins.iter().any(|o| o == "*") {
        tower_http::cors::AllowOrigin::any()
    } else {
        let origins = config
            .kora
            .cors_allow_origins
            .iter()
            .filter_map(|o| {
                o.parse::<http::HeaderValue>()
                    .map_err(|e| log::warn!("Invalid CORS origin '{}': {}", o, e))
                    .ok()
            })
            .collect::<Vec<_>>();
        tower_http::cors::AllowOrigin::list(origins)
    };

    // Build middleware stack with tracing and CORS
    let cors = CorsLayer::new()
        .allow_origin(allow_origins)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static(X_API_KEY),
            header::HeaderName::from_static(X_HMAC_SIGNATURE),
            header::HeaderName::from_static(X_RECAPTCHA_TOKEN),
            header::HeaderName::from_static(X_TIMESTAMP),
        ])
        .max_age(Duration::from_secs(3600));

    // Get the RPC client from KoraRpc to pass to metrics initialization
    let rpc_client = rpc.get_rpc_client().clone();

    let (metrics_handle, metrics_layers, balance_tracker_handle) =
        run_metrics_server_if_required(port, rpc_client).await?;

    // Build whitelist of allowed methods from enabled_methods config
    let allowed_methods = config.kora.enabled_methods.get_enabled_method_names();

    let recaptcha_config =
        get_value_by_priority("KORA_RECAPTCHA_SECRET", config.kora.auth.recaptcha_secret.clone())
            .map(|secret| {
                RecaptchaConfig::new(
                    secret,
                    config.kora.auth.recaptcha_score_threshold,
                    config.kora.auth.protected_methods.clone(),
                )
            });

    let api_keys_config = std::env::var("KORA_API_KEY")
        .ok()
        .map(|k| vec![k])
        .or_else(|| config.kora.auth.api_keys.clone());

    let middleware = tower::ServiceBuilder::new()
        // Add metrics handler first (before other layers) so it can intercept /metrics
        .layer(ProxyGetRequestLayer::new("/liveness", "liveness")?)
        // Add metrics handler layer for Prometheus metrics
        .option_layer(
            metrics_layers.as_ref().and_then(|layers| layers.metrics_handler_layer.clone()),
        )
        // Add metrics collection layer
        .option_layer(metrics_layers.as_ref().and_then(|layers| layers.http_metrics_layer.clone()))
        // cors
        .layer(cors)
        // Method validation layer - to fail fast
        .layer(MethodValidationLayer::new(allowed_methods.clone()))
        // Add authentication layer for API key if configured
        .option_layer(api_keys_config.map(ApiKeyAuthLayer::new))
        // Add authentication layer for HMAC if configured
        .option_layer(
            get_value_by_priority("KORA_HMAC_SECRET", config.kora.auth.hmac_secret.clone())
                .map(|secret| HmacAuthLayer::new(secret, config.kora.auth.max_timestamp_age)),
        )
        // Identity-aware rate limiting
        .layer(IdentityRateLimitLayer::new(config.kora.rate_limit))
        // Add reCAPTCHA verification layer if configured
        .option_layer(recaptcha_config.map(RecaptchaLayer::new));

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
        constant::X_API_KEY,
        tests::{
            common::setup_or_get_test_signer,
            config_mock::{AuthConfigBuilder, ConfigMockBuilder, KoraConfigBuilder},
            rpc_mock::RpcMockBuilder,
        },
    };
    use reqwest::Client;
    use std::{env, net::TcpListener, time::Instant};

    #[tokio::test]
    async fn test_identity_rate_limit_behaviors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let auth_config = AuthConfigBuilder::new()
            .with_api_keys(vec!["key-a".to_string(), "key-b".to_string()])
            .build();
        // Set limit to 1 request per second
        let kora_config =
            KoraConfigBuilder::new().with_rate_limit(1).with_auth(auth_config).build();
        let _m = ConfigMockBuilder::new().with_kora(kora_config).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = RpcMockBuilder::new().build();
        let _handles =
            run_rpc_server(KoraRpc::new(rpc_client), port).await.expect("Failed to start server");

        let client = Client::new();
        let url = format!("http://127.0.0.1:{}", port);

        // --- Test 1: Independent identities don't throttle each other ---

        // Request 1: Valid key-a
        let res1 = client
            .post(&url)
            .header(X_API_KEY, "key-a")
            .header("content-type", "application/json")
            .body(r#"{"jsonrpc":"2.0","method":"getConfig","params":[],"id":1}"#)
            .send()
            .await
            .expect("Failed to send request");
        assert_eq!(res1.status(), reqwest::StatusCode::OK);

        let start = Instant::now();

        // Request 2: Different valid key-b
        let res2 = client
            .post(&url)
            .header(X_API_KEY, "key-b")
            .header("content-type", "application/json")
            .body(r#"{"jsonrpc":"2.0","method":"getConfig","params":[],"id":2}"#)
            .send()
            .await
            .expect("Failed to send request");

        // Identity rate limiter uses separate buckets; key-b does NOT wait for key-a's usage
        assert!(
            start.elapsed().as_millis() < 200,
            "Expected no throttling delay for independent identity"
        );
        assert_eq!(res2.status(), reqwest::StatusCode::OK);

        // --- Test 2: Same identity is throttled correctly ---

        // Request 3: key-a AGAIN, should be rate limited immediately (HTTP 429)
        let res3 = client
            .post(&url)
            .header(X_API_KEY, "key-a")
            .header("content-type", "application/json")
            .body(r#"{"jsonrpc":"2.0","method":"getConfig","params":[],"id":3}"#)
            .send()
            .await
            .expect("Failed to send request");
        assert_eq!(res3.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_identity_rate_limit_zero_bypasses_limiter() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let auth_config =
            AuthConfigBuilder::new().with_api_keys(vec!["key-zero".to_string()]).build();
        // Set limit to 0 (disabled)
        let kora_config =
            KoraConfigBuilder::new().with_rate_limit(0).with_auth(auth_config).build();
        let _m = ConfigMockBuilder::new().with_kora(kora_config).build_and_setup();
        let _ = setup_or_get_test_signer();

        let rpc_client = RpcMockBuilder::new().build();
        let _handles =
            run_rpc_server(KoraRpc::new(rpc_client), port).await.expect("Failed to start server");

        let client = Client::new();
        let url = format!("http://127.0.0.1:{}", port);

        // Send 10 rapid requests
        for i in 1..=10 {
            let res = client
                .post(&url)
                .header(X_API_KEY, "key-zero")
                .header("content-type", "application/json")
                .body(format!(r#"{{"jsonrpc":"2.0","method":"getConfig","params":[],"id":{}}}"#, i))
                .send()
                .await
                .expect("Failed to send request");
            assert_eq!(res.status(), reqwest::StatusCode::OK);
        }
    }

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
    fn test_get_value_by_priority_empty_env_var_falls_back_to_config() {
        let env_var_name = "TEST_ENV_VAR_EMPTY_ENV_UNIQUE_DEF456";
        env::set_var(env_var_name, "");

        let result = get_value_by_priority(env_var_name, Some("config_value".to_string()));
        assert_eq!(result, Some("config_value".to_string()));

        env::remove_var(env_var_name);
    }

    #[test]
    fn test_get_value_by_priority_empty_config_value_is_ignored() {
        let env_var_name = "TEST_ENV_VAR_EMPTY_CONFIG_UNIQUE_GHI789";

        let result = get_value_by_priority(env_var_name, Some("".to_string()));
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
