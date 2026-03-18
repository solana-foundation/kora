use crate::{
    constant::X_RECAPTCHA_TOKEN,
    rpc_server::{
        middleware_utils::{extract_parts_and_body_bytes, get_jsonrpc_method},
        recaptcha_util::RecaptchaConfig,
    },
};
use http::{Request, Response};
use jsonrpsee::server::logger::Body;

#[derive(Clone)]
pub struct RecaptchaLayer {
    config: RecaptchaConfig,
}

impl RecaptchaLayer {
    pub fn new(config: RecaptchaConfig) -> Self {
        Self { config }
    }
}

impl<S> tower::Layer<S> for RecaptchaLayer {
    type Service = RecaptchaService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RecaptchaService { inner, config: self.config.clone() }
    }
}

#[derive(Clone)]
pub struct RecaptchaService<S> {
    inner: S,
    config: RecaptchaConfig,
}

impl<S> tower::Service<Request<Body>> for RecaptchaService<S>
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
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            if let Some(method) = get_jsonrpc_method(&body_bytes) {
                if !config.is_protected_method(&method) {
                    let new_request = Request::from_parts(parts, Body::from(body_bytes));
                    return inner.call(new_request).await;
                }
            }

            let new_request = Request::from_parts(parts, Body::from(body_bytes));
            let recaptcha_token =
                new_request.headers().get(X_RECAPTCHA_TOKEN).and_then(|v| v.to_str().ok());

            if let Err(resp) = config.validate(recaptcha_token).await {
                return Ok(resp);
            }

            inner.call(new_request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Method, StatusCode};
    use std::{
        future::Ready,
        task::{Context, Poll},
    };
    use tower::{Layer, Service, ServiceExt};

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

    fn test_recaptcha_config() -> RecaptchaConfig {
        RecaptchaConfig::new(
            "test-recaptcha-secret".to_string(),
            0.5,
            vec!["signTransaction".to_string(), "signAndSendTransaction".to_string()],
        )
    }

    #[tokio::test]
    async fn test_recaptcha_layer_bypasses_unprotected_method() {
        let config = test_recaptcha_config();
        let layer = RecaptchaLayer::new(config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_recaptcha_layer_rejects_protected_method_missing_token() {
        let config = test_recaptcha_config();
        let layer = RecaptchaLayer::new(config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_recaptcha_layer_rejects_sign_and_send_missing_token() {
        let config = test_recaptcha_config();
        let layer = RecaptchaLayer::new(config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signAndSendTransaction","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_recaptcha_layer_rejects_empty_token() {
        let config = test_recaptcha_config();
        let layer = RecaptchaLayer::new(config);
        let mut service = layer.layer(MockService);

        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header(X_RECAPTCHA_TOKEN, "")
            .body(Body::from(body))
            .unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
