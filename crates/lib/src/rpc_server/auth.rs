use crate::{
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP},
    rpc_server::middleware_utils::{
        build_response_with_graceful_error, extract_parts_and_body_bytes, get_jsonrpc_method,
    },
};
use hmac::{Hmac, KeyInit, Mac};
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use sha2::Sha256;
use subtle::ConstantTimeEq;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientIdentity(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectionReason {
    AuthFailure,
    RateLimit,
}

impl RejectionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RejectionReason::AuthFailure => "auth_failure",
            RejectionReason::RateLimit => "rate_limit",
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthLayer {
    api_keys: Vec<String>,
}

impl ApiKeyAuthLayer {
    pub fn new(api_keys: Vec<String>) -> Self {
        Self { api_keys }
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthService<S> {
    inner: S,
    api_keys: Vec<String>,
}

impl<S> tower::Layer<S> for ApiKeyAuthLayer {
    type Service = ApiKeyAuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyAuthService { inner, api_keys: self.api_keys.clone() }
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
        let api_keys = self.api_keys.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut unauthorized_response =
                build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, "");
            unauthorized_response.extensions_mut().insert(RejectionReason::AuthFailure);

            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            // Bypass auth for liveness endpoint
            if let Some(method) = get_jsonrpc_method(&body_bytes) {
                if method == "liveness" {
                    let new_body = Body::from(body_bytes);
                    let new_request = Request::from_parts(parts, new_body);
                    return inner.call(new_request).await;
                }
            }

            // Check for API key header
            let mut req = Request::from_parts(parts, Body::from(body_bytes));
            let provided_key = req.headers().get(X_API_KEY).cloned();
            if let Some(provided_key) = provided_key {
                // Constant-time comparison prevents timing attacks
                let mut matched = false;
                for key in &api_keys {
                    let is_match: bool = provided_key.as_bytes().ct_eq(key.as_bytes()).into();
                    matched |= is_match;
                }

                if matched {
                    if let Ok(key_str) = provided_key.to_str() {
                        req.extensions_mut().insert(ClientIdentity(format!("apikey:{}", key_str)));
                    }
                    return inner.call(req).await;
                }
            }

            Ok(unauthorized_response)
        })
    }
}

#[derive(Clone)]
pub struct HmacAuthLayer {
    secret: String,
    max_timestamp_age: i64,
}

impl HmacAuthLayer {
    pub fn new(secret: String, max_timestamp_age: i64) -> Self {
        Self { secret, max_timestamp_age }
    }
}

impl<S> tower::Layer<S> for HmacAuthLayer {
    type Service = HmacAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HmacAuthService {
            inner,
            secret: self.secret.clone(),
            max_timestamp_age: self.max_timestamp_age,
        }
    }
}

#[derive(Clone)]
pub struct HmacAuthService<S> {
    inner: S,
    secret: String,
    max_timestamp_age: i64,
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
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut unauthorized_response =
                build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, "");
            unauthorized_response.extensions_mut().insert(RejectionReason::AuthFailure);

            let signature_header = request.headers().get(X_HMAC_SIGNATURE).cloned();
            let timestamp_header = request.headers().get(X_TIMESTAMP).cloned();

            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            // Bypass auth for liveness endpoint
            if let Some(method) = get_jsonrpc_method(&body_bytes) {
                if method == "liveness" {
                    let new_body = Body::from(body_bytes);
                    let new_request = Request::from_parts(parts, new_body);
                    return inner.call(new_request).await;
                }
            }

            let (signature, timestamp) = match (signature_header, timestamp_header) {
                (Some(sig), Some(ts)) => {
                    let sig_str = match sig.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => return Ok(unauthorized_response),
                    };
                    let ts_str = match ts.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => return Ok(unauthorized_response),
                    };
                    (sig_str, ts_str)
                }
                _ => return Ok(unauthorized_response),
            };

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

            // Reconstruct the request with the consumed body
            let new_body = Body::from(body_bytes);
            let mut new_request = Request::from_parts(parts, new_body);
            // HMAC currently uses a single global shared secret, so all HMAC clients share one rate-limit bucket
            new_request.extensions_mut().insert(ClientIdentity("hmac:global".to_string()));

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
        convert::Infallible,
        future::Ready,
        task::{Context, Poll},
    };
    use tower::{Layer, Service, ServiceExt};

    // Mock service that always returns OK
    #[derive(Clone)]
    struct MockService;

    impl Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request<Body>) -> Self::Future {
            let mut res = Response::builder().status(200).body(Body::empty()).unwrap();
            if let Some(identity) = req.extensions().get::<ClientIdentity>() {
                res.extensions_mut().insert(identity.clone());
            }
            std::future::ready(Ok(res))
        }
    }

    #[tokio::test]
    async fn test_api_key_auth_valid_key() {
        let layer = ApiKeyAuthLayer::new(vec!["test-key".to_string()]);
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder()
            .uri("/test")
            .header(X_API_KEY, "test-key")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.extensions().get::<ClientIdentity>().unwrap().0, "apikey:test-key");
    }

    #[tokio::test]
    async fn test_api_key_auth_invalid_key() {
        let layer = ApiKeyAuthLayer::new(vec!["test-key".to_string()]);
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
        let layer = ApiKeyAuthLayer::new(vec!["test-key".to_string()]);
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request = Request::builder().uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_auth_liveness_bypass() {
        let layer = ApiKeyAuthLayer::new(vec!["test-key".to_string()]);
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
        assert_eq!(response.extensions().get::<ClientIdentity>().unwrap().0, "hmac:global");
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
        assert_eq!(
            response.extensions().get::<RejectionReason>(),
            Some(&RejectionReason::AuthFailure)
        );
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
        assert_eq!(
            response.extensions().get::<RejectionReason>(),
            Some(&RejectionReason::AuthFailure)
        );
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
        assert_eq!(
            response.extensions().get::<RejectionReason>(),
            Some(&RejectionReason::AuthFailure)
        );
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
}
