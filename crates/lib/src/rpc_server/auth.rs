use crate::{
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_RECAPTCHA_TOKEN, X_TIMESTAMP},
    rpc_server::{
        middleware_utils::{
            build_response_with_graceful_error, extract_parts_and_body_bytes, get_jsonrpc_method,
        },
        recaptcha_util::{RecaptchaConfig, RecaptchaValidated},
    },
};
use hmac::{Hmac, Mac};
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use sha2::Sha256;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct ApiKeyAuthLayer {
    api_key: String,
    recaptcha_config: Option<RecaptchaConfig>,
}

impl ApiKeyAuthLayer {
    pub fn new(api_key: String) -> Self {
        Self { api_key, recaptcha_config: None }
    }

    pub fn with_recaptcha(mut self, recaptcha_config: Option<RecaptchaConfig>) -> Self {
        self.recaptcha_config = recaptcha_config;
        self
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthService<S> {
    inner: S,
    api_key: String,
    recaptcha_config: Option<RecaptchaConfig>,
}

impl<S> tower::Layer<S> for ApiKeyAuthLayer {
    type Service = ApiKeyAuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyAuthService {
            inner,
            api_key: self.api_key.clone(),
            recaptcha_config: self.recaptcha_config.clone(),
        }
    }
}

impl<S> tower::Service<Request<Body>> for ApiKeyAuthService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let api_key = self.api_key.clone();
        let recaptcha_config = self.recaptcha_config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let unauthorized_response =
                build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, "");

            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            let method = get_jsonrpc_method(&body_bytes).unwrap_or_default();

            // Bypass auth for liveness endpoint
            if method == "liveness" {
                let new_body = Body::from(body_bytes);
                let new_request = Request::from_parts(parts, new_body);
                return inner.call(new_request).await;
            }

            // Check for API key header
            let mut req = Request::from_parts(parts, Body::from(body_bytes.clone()));
            if let Some(provided_key) = req.headers().get(X_API_KEY) {
                // Constant-time comparison prevents timing attacks
                if provided_key.as_bytes().ct_eq(api_key.as_bytes()).into() {
                    if let Some(config) = &recaptcha_config {
                        if req.extensions().get::<RecaptchaValidated>().is_none() {
                            let recaptcha_token =
                                req.headers().get(X_RECAPTCHA_TOKEN).and_then(|v| v.to_str().ok());
                            if let Err(resp) = config.validate(recaptcha_token, &method).await {
                                return Ok(resp);
                            }
                            req.extensions_mut().insert(RecaptchaValidated);
                        }
                    }
                    return inner.call(req).await;
                }
            }

            Ok(unauthorized_response)
        })
    }
}

// ============ HMAC Authentication ============

#[derive(Clone)]
pub struct HmacAuthLayer {
    secret: String,
    max_timestamp_age: i64,
    recaptcha_config: Option<RecaptchaConfig>,
}

impl HmacAuthLayer {
    pub fn new(secret: String, max_timestamp_age: i64) -> Self {
        Self { secret, max_timestamp_age, recaptcha_config: None }
    }

    pub fn with_recaptcha(mut self, recaptcha_config: Option<RecaptchaConfig>) -> Self {
        self.recaptcha_config = recaptcha_config;
        self
    }
}

impl<S> tower::Layer<S> for HmacAuthLayer {
    type Service = HmacAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HmacAuthService {
            inner,
            secret: self.secret.clone(),
            max_timestamp_age: self.max_timestamp_age,
            recaptcha_config: self.recaptcha_config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct HmacAuthService<S> {
    inner: S,
    secret: String,
    max_timestamp_age: i64,
    recaptcha_config: Option<RecaptchaConfig>,
}

impl<S> tower::Service<Request<Body>> for HmacAuthService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let secret = self.secret.clone();
        let max_timestamp_age = self.max_timestamp_age;
        let recaptcha_config = self.recaptcha_config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let unauthorized_response =
                build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, "");

            let signature_header = request.headers().get(X_HMAC_SIGNATURE).cloned();
            let timestamp_header = request.headers().get(X_TIMESTAMP).cloned();

            let (mut parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            // Get method name for bypass checks
            let method = get_jsonrpc_method(&body_bytes).unwrap_or_default();

            // Bypass auth for liveness endpoint
            if method == "liveness" {
                let new_body = Body::from(body_bytes);
                let new_request = Request::from_parts(parts, new_body);
                return inner.call(new_request).await;
            }

            let (signature, timestamp) =
                match (signature_header.as_ref(), timestamp_header.as_ref()) {
                    (Some(sig), Some(ts)) => (sig, ts),
                    _ => return Ok(unauthorized_response),
                };

            let signature = signature.to_str().unwrap_or("");
            let timestamp = timestamp.to_str().unwrap_or("");

            // Verify timestamp is within allowed age
            let ts = match timestamp.parse::<i64>() {
                Ok(ts) => ts,
                Err(_) => return Ok(unauthorized_response),
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| {
                    log::error!("System time error: {e:?}");
                    e
                })
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs() as i64;

            if (now - ts).abs() > max_timestamp_age {
                return Ok(unauthorized_response);
            }

            // Verify HMAC signature using timestamp + body
            let body_str = match std::str::from_utf8(&body_bytes) {
                Ok(s) => s,
                Err(_) => {
                    log::error!("HMAC authentication failed: invalid UTF-8 in request body");
                    return Ok(unauthorized_response);
                }
            };
            let message = format!("{}{}", timestamp, body_str);

            let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                Ok(mac) => mac,
                Err(_) => {
                    log::error!("HMAC authentication failed");
                    return Ok(unauthorized_response);
                }
            };

            mac.update(message.as_bytes());

            let signature_bytes = match hex::decode(signature) {
                Ok(bytes) => bytes,
                Err(_) => {
                    log::error!("HMAC signature hex decode failed");
                    return Ok(unauthorized_response);
                }
            };

            // Constant time comparison prevents timing attacks
            if mac.verify_slice(&signature_bytes).is_err() {
                return Ok(unauthorized_response);
            }

            if let Some(config) = &recaptcha_config {
                if parts.extensions.get::<RecaptchaValidated>().is_none() {
                    let recaptcha_token =
                        parts.headers.get(X_RECAPTCHA_TOKEN).and_then(|v| v.to_str().ok());
                    if let Err(resp) = config.validate(recaptcha_token, &method).await {
                        return Ok(resp);
                    }
                    parts.extensions.insert(RecaptchaValidated);
                }
            }

            // Reconstruct the request with the consumed body
            let new_body = Body::from(body_bytes);
            let new_request = Request::from_parts(parts, new_body);

            inner.call(new_request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constant::{DEFAULT_MAX_TIMESTAMP_AGE, X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP};
    use hmac::{Hmac, Mac};
    use http::Method;
    use jsonrpsee::server::logger::Body;
    use sha2::Sha256;
    use std::{
        future::Ready,
        task::{Context, Poll},
    };
    use tower::{Layer, Service, ServiceExt};

    // Mock service that always returns OK
    #[derive(Clone)]
    struct MockService;

    impl tower::Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = std::convert::Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            std::future::ready(Ok(Response::builder().status(200).body(Body::empty()).unwrap()))
        }
    }

    #[tokio::test]
    async fn test_api_key_auth_valid_key() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder()
            .uri("/test")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_auth_invalid_key() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder()
            .uri("/test")
            .header(X_API_KEY, "wrong-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_auth_missing_header() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder().uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_auth_liveness_bypass() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let liveness_body = r#"{"jsonrpc":"2.0","method":"liveness","params":[],"id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .body(Body::from(liveness_body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hmac_auth_valid_signature() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let message = format!("{timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hmac_auth_invalid_signature() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, "invalid-signature")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_hmac_auth_missing_headers() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_hmac_auth_expired_timestamp() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        // Timestamp from 10 minutes ago (expired)
        let expired_timestamp =
            (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                - 600)
                .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let message = format!("{expired_timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header(X_TIMESTAMP, &expired_timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_hmac_auth_malformed_timestamp() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header(X_TIMESTAMP, "not-a-number")
            .header(X_HMAC_SIGNATURE, "some-signature")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_hmac_auth_liveness_bypass() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let liveness_body = r#"{"jsonrpc":"2.0","method":"liveness","params":[],"id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .body(Body::from(liveness_body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    fn test_recaptcha_config() -> Option<RecaptchaConfig> {
        Some(RecaptchaConfig::new(
            "test-recaptcha-secret".to_string(),
            0.5,
            vec!["signTransaction".to_string(), "signAndSendTransaction".to_string()],
        ))
    }

    #[tokio::test]
    async fn test_api_key_with_recaptcha_unprotected_method_bypasses() {
        let recaptcha_config = test_recaptcha_config();
        let layer = ApiKeyAuthLayer::new("test-key".to_string()).with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_with_recaptcha_protected_method_missing_token() {
        let recaptcha_config = test_recaptcha_config();
        let layer = ApiKeyAuthLayer::new("test-key".to_string()).with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_with_recaptcha_sign_and_send_missing_token() {
        let recaptcha_config = test_recaptcha_config();
        let layer = ApiKeyAuthLayer::new("test-key".to_string()).with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signAndSendTransaction","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_with_recaptcha_empty_token_rejected() {
        let recaptcha_config = test_recaptcha_config();
        let layer = ApiKeyAuthLayer::new("test-key".to_string()).with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .header(X_RECAPTCHA_TOKEN, "")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_without_recaptcha_config_allows_protected_methods() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hmac_with_recaptcha_unprotected_method_bypasses() {
        let secret = "test-secret";
        let recaptcha_config = test_recaptcha_config();
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE)
            .with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let message = format!("{timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hmac_with_recaptcha_protected_method_missing_token() {
        let secret = "test-secret";
        let recaptcha_config = test_recaptcha_config();
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE)
            .with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let message = format!("{timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_hmac_without_recaptcha_config_allows_protected_methods() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let message = format!("{timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_skips_recaptcha_if_already_validated() {
        let recaptcha_config = test_recaptcha_config();
        let layer = ApiKeyAuthLayer::new("test-key".to_string()).with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        request.extensions_mut().insert(RecaptchaValidated);

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_hmac_skips_recaptcha_if_already_validated() {
        let secret = "test-secret";
        let recaptcha_config = test_recaptcha_config();
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE)
            .with_recaptcha(recaptcha_config);
        let mut service = layer.layer(MockService);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let message = format!("{timestamp}{body}");

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_TIMESTAMP, &timestamp)
            .header(X_HMAC_SIGNATURE, &signature)
            .body(Body::from(body))
            .unwrap();

        request.extensions_mut().insert(RecaptchaValidated);

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
