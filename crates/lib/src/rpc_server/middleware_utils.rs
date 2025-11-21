use std::collections::{HashMap, HashSet};

use futures_util::TryStreamExt;
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;

use crate::KoraError;

pub fn default_sig_verify() -> bool {
    false
}

pub async fn extract_parts_and_body_bytes(
    request: Request<Body>,
) -> (http::request::Parts, Vec<u8>) {
    let (parts, body) = request.into_parts();
    let body_bytes = body
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap_or_default();
    (parts, body_bytes)
}

pub fn get_jsonrpc_method(body_bytes: &[u8]) -> Option<String> {
    match serde_json::from_slice::<serde_json::Value>(body_bytes) {
        Ok(val) => val.get("method").and_then(|m| m.as_str()).map(|s| s.to_string()),
        Err(_) => None,
    }
}

pub fn verify_jsonrpc_method(
    body_bytes: &[u8],
    allowed_methods: &HashSet<String>,
) -> Result<String, KoraError> {
    let method = get_jsonrpc_method(body_bytes);
    if let Some(method) = method {
        if allowed_methods.contains(&method) {
            return Ok(method);
        }
    }
    Err(KoraError::InvalidRequest("Method not allowed".to_string()))
}

pub fn build_response_with_graceful_error(
    headers: Option<HashMap<String, String>>,
    status_code: StatusCode,
    error_message: &str,
) -> Response<Body> {
    let mut builder = Response::builder();

    if let Some(headers) = headers {
        for (key, value) in headers.iter() {
            builder = builder.header(key, value);
        }
    }

    builder.status(status_code).body(Body::from(error_message.to_string())).unwrap_or_else(|e| {
        log::error!("Failed to build response, error: {e:?}");
        let mut response = Response::new(Body::empty());
        *response.status_mut() = status_code;
        response
    })
}

/// Method validation layer - applies first in middleware stack to fail fast
#[derive(Clone)]
pub struct MethodValidationLayer {
    allowed_methods: HashSet<String>,
}

impl MethodValidationLayer {
    pub fn new(allowed_methods: Vec<String>) -> Self {
        Self { allowed_methods: allowed_methods.into_iter().collect() }
    }
}

#[derive(Clone)]
pub struct MethodValidationService<S> {
    inner: S,
    allowed_methods: HashSet<String>,
}

impl<S> tower::Layer<S> for MethodValidationLayer {
    type Service = MethodValidationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MethodValidationService { inner, allowed_methods: self.allowed_methods.clone() }
    }
}

impl<S> tower::Service<Request<Body>> for MethodValidationService<S>
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
        let allowed_methods = self.allowed_methods.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            match verify_jsonrpc_method(&body_bytes, &allowed_methods) {
                Ok(_) => {}
                Err(_) => {
                    return Ok(build_response_with_graceful_error(
                        None,
                        StatusCode::METHOD_NOT_ALLOWED,
                        "",
                    ));
                }
            }

            let new_body = Body::from(body_bytes);
            let new_request = Request::from_parts(parts, new_body);
            inner.call(new_request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
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
    async fn test_method_validation_disallowed_method() {
        let allowed_methods = vec!["liveness".to_string(), "getConfig".to_string()];
        let layer = MethodValidationLayer::new(allowed_methods);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"unknownMethod","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_method_validation_malformed_json() {
        let allowed_methods = vec!["liveness".to_string(), "getConfig".to_string()];
        let layer = MethodValidationLayer::new(allowed_methods);
        let mut service = layer.layer(MockService);

        let body = r#"{"invalid json"#;
        let request =
            Request::builder().method(Method::POST).uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_method_validation_missing_method_field() {
        let allowed_methods = vec!["liveness".to_string(), "getConfig".to_string()];
        let layer = MethodValidationLayer::new(allowed_methods);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/test").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_method_validation_multiple_allowed_methods() {
        let allowed_methods = vec![
            "liveness".to_string(),
            "getConfig".to_string(),
            "signTransaction".to_string(),
            "estimateTransactionFee".to_string(),
        ];
        let layer = MethodValidationLayer::new(allowed_methods);
        let mut service = layer.layer(MockService);

        // Test each allowed method
        for method in &["liveness", "getConfig", "signTransaction", "estimateTransactionFee"] {
            let body = format!(r#"{{"jsonrpc":"2.0","method":"{}","id":1}}"#, method);
            let request = Request::builder()
                .method(Method::POST)
                .uri("/test")
                .body(Body::from(body))
                .unwrap();

            let response = service.ready().await.unwrap().call(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "Method {} should be allowed", method);
        }
    }
}
