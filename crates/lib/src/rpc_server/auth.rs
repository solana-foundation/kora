use crate::{
    constant::{X_API_KEY, X_HMAC_SIGNATURE, X_TIMESTAMP},
    rpc_server::middleware_utils::{extract_parts_and_body_bytes, get_jsonrpc_method},
};
use hmac::{Hmac, Mac};
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use sha2::Sha256;

#[derive(Clone)]
pub struct ApiKeyAuthLayer {
    api_key: String,
}

impl ApiKeyAuthLayer {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Clone)]
pub struct ApiKeyAuthService<S> {
    inner: S,
    api_key: String,
}

impl<S> tower::Layer<S> for ApiKeyAuthLayer {
    type Service = ApiKeyAuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyAuthService { inner, api_key: self.api_key.clone() }
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
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let unauthorized_response =
                Response::builder().status(StatusCode::UNAUTHORIZED).body(Body::empty()).unwrap();

            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;
            if get_jsonrpc_method(&body_bytes) == Some("liveness".to_string()) {
                let new_body = Body::from(body_bytes);
                let new_request = Request::from_parts(parts, new_body);
                return inner.call(new_request).await;
            }

            let req = Request::from_parts(parts, Body::from(body_bytes));
            if let Some(provided_key) = req.headers().get(X_API_KEY) {
                if provided_key.to_str().unwrap_or("") == api_key {
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
            let unauthorized_response =
                Response::builder().status(StatusCode::UNAUTHORIZED).body(Body::empty()).unwrap();

            let signature_header = request.headers().get(X_HMAC_SIGNATURE).cloned();
            let timestamp_header = request.headers().get(X_TIMESTAMP).cloned();

            // Since our proxy for get /liveness transforms the request in a POST, we need to check the liveness via the method in the body
            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;
            if get_jsonrpc_method(&body_bytes) == Some("liveness".to_string()) {
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

            // Verify timestamp is within 5 minutes
            let parsed_timestamp = timestamp.parse::<i64>();
            if parsed_timestamp.is_err() {
                return Ok(unauthorized_response);
            }

            let ts = parsed_timestamp.expect("timestamp already validated");
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
            let body_str = std::str::from_utf8(&body_bytes).unwrap_or("");
            let message = format!("{}{}", timestamp, body_str);

            let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                Ok(mac) => mac,
                Err(e) => {
                    log::error!("Invalid HMAC secret: {e:?}");
                    return Ok(unauthorized_response);
                }
            };

            mac.update(message.as_bytes());
            let expected_signature = hex::encode(mac.finalize().into_bytes());

            if signature != expected_signature {
                return Ok(unauthorized_response);
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
        let request = Request::builder()
            .uri("/test")
            .header(X_API_KEY, "test-key")
            .body(Body::empty())
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_auth_invalid_key() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let request = Request::builder()
            .uri("/test")
            .header(X_API_KEY, "wrong-key")
            .body(Body::empty())
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_auth_missing_header() {
        let layer = ApiKeyAuthLayer::new("test-key".to_string());
        let mut service = layer.layer(MockService);
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

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

        let body = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
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

        let body = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;

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

        let request =
            Request::builder().method(Method::POST).uri("/test").body(Body::from("test")).unwrap();

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

        let body = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
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

    #[tokio::test]
    async fn test_hmac_auth_malformed_timestamp() {
        let secret = "test-secret";
        let layer = HmacAuthLayer::new(secret.to_string(), DEFAULT_MAX_TIMESTAMP_AGE);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;

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
}
