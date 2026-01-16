use crate::{
    constant::{DEFAULT_PROTECTED_METHODS, RECAPTCHA_VERIFY_URL, X_RECAPTCHA_TOKEN},
    error::KoraError,
    rpc_server::middleware_utils::{
        build_response_with_graceful_error, extract_parts_and_body_bytes, get_jsonrpc_method,
    },
    sanitize_error,
};
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RecaptchaVerifyResponse {
    success: bool,
    score: Option<f64>,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct RecaptchaAuthLayer {
    secret: String,
    score_threshold: f64,
    http_client: Client,
}

impl RecaptchaAuthLayer {
    pub fn new(secret: String, score_threshold: f64) -> Self {
        Self { secret, score_threshold, http_client: Client::new() }
    }
}

impl<S> tower::Layer<S> for RecaptchaAuthLayer {
    type Service = RecaptchaAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RecaptchaAuthService {
            inner,
            secret: self.secret.clone(),
            score_threshold: self.score_threshold,
            http_client: self.http_client.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RecaptchaAuthService<S> {
    inner: S,
    secret: String,
    score_threshold: f64,
    http_client: Client,
}

impl<S> tower::Service<Request<Body>> for RecaptchaAuthService<S>
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
        let score_threshold = self.score_threshold;
        let http_client = self.http_client.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let unauthorized_response =
                build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, "");

            let recaptcha_token_header = request.headers().get(X_RECAPTCHA_TOKEN).cloned();

            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            if let Some(method) = get_jsonrpc_method(&body_bytes) {
                // Bypass for non-protected methods
                if !DEFAULT_PROTECTED_METHODS.contains(&method.as_str()) {
                    let new_body = Body::from(body_bytes);
                    let new_request = Request::from_parts(parts, new_body);
                    return inner.call(new_request).await;
                }
            }

            let recaptcha_token = match recaptcha_token_header.as_ref() {
                Some(token) => token,
                _ => {
                    return Ok(unauthorized_response);
                }
            };

            let token = recaptcha_token.to_str().unwrap_or("");

            match verify_recaptcha(&http_client, RECAPTCHA_VERIFY_URL, &secret, token).await {
                Ok(score) => {
                    if score < score_threshold {
                        log::warn!(
                            "reCAPTCHA verification failed: score {:.2} below threshold {:.2}",
                            score,
                            score_threshold
                        );
                        return Ok(unauthorized_response);
                    }
                }
                Err(e) => {
                    log::error!("reCAPTCHA verification error: {}", e);
                    return Ok(unauthorized_response);
                }
            }

            let new_body = Body::from(body_bytes);
            let new_request = Request::from_parts(parts, new_body);
            inner.call(new_request).await
        })
    }
}

async fn verify_recaptcha(
    client: &Client,
    verify_url: &str,
    secret: &str,
    token: &str,
) -> Result<f64, KoraError> {
    let response = client
        .post(verify_url)
        .form(&[("secret", secret), ("response", token)])
        .send()
        .await
        .map_err(|e| {
            KoraError::RecaptchaError(format!("API call failed: {}", sanitize_error!(e)))
        })?;

    if !response.status().is_success() {
        return Err(KoraError::RecaptchaError(format!(
            "API returned status: {}",
            response.status()
        )));
    }

    let verify_response: RecaptchaVerifyResponse = response.json().await.map_err(|e| {
        KoraError::RecaptchaError(format!("Failed to parse response: {}", sanitize_error!(e)))
    })?;

    if !verify_response.success {
        let errors = verify_response.error_codes.unwrap_or_default().join(", ");
        return Err(KoraError::RecaptchaError(format!("Verification failed: {}", errors)));
    }

    verify_response
        .score
        .ok_or_else(|| KoraError::RecaptchaError("Response missing score".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use mockito::Server;
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

    // ========== Layer/Service Tests ==========

    #[tokio::test]
    async fn test_recaptcha_bypass_unprotected_method() {
        let layer = RecaptchaAuthLayer::new("test-secret".to_string(), 0.5);
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"getConfig","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_recaptcha_missing_token_protected_method() {
        let layer = RecaptchaAuthLayer::new("test-secret".to_string(), 0.5);
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"signTransaction","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_recaptcha_missing_token_sign_and_send() {
        let layer = RecaptchaAuthLayer::new("test-secret".to_string(), 0.5);
        let mut service = layer.layer(MockService);
        let body = r#"{"jsonrpc":"2.0","method":"signAndSendTransaction","id":1}"#;
        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========== verify_recaptcha Unit Tests ==========

    #[tokio::test]
    async fn test_verify_recaptcha_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true, "score": 0.85}"#)
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_ok());
        assert!((result.unwrap() - 0.85).abs() < f64::EPSILON);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(503)
            .with_body("Service Unavailable")
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::RecaptchaError(_)));
        if let KoraError::RecaptchaError(msg) = error {
            assert!(msg.contains("API returned status"));
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_invalid_json_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::RecaptchaError(_)));
        if let KoraError::RecaptchaError(msg) = error {
            assert!(msg.contains("Failed to parse response"));
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_verification_failed_with_error_codes() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": false, "error-codes": ["invalid-input-secret", "timeout-or-duplicate"]}"#)
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::RecaptchaError(_)));
        if let KoraError::RecaptchaError(msg) = error {
            assert!(msg.contains("Verification failed"));
            assert!(msg.contains("invalid-input-secret"));
            assert!(msg.contains("timeout-or-duplicate"));
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_missing_score() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true}"#)
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::RecaptchaError(_)));
        if let KoraError::RecaptchaError(msg) = error {
            assert!(msg.contains("missing score"));
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_verification_failed_no_error_codes() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": false}"#)
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KoraError::RecaptchaError(_)));
        if let KoraError::RecaptchaError(msg) = error {
            assert!(msg.contains("Verification failed"));
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_verify_recaptcha_score_threshold_boundary() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/recaptcha/api/siteverify")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true, "score": 0.5}"#)
            .create_async()
            .await;

        let client = Client::new();
        let verify_url = format!("{}/recaptcha/api/siteverify", server.url());

        let result = verify_recaptcha(&client, &verify_url, "secret", "token").await;
        assert!(result.is_ok());
        assert!((result.unwrap() - 0.5).abs() < f64::EPSILON);
        mock.assert_async().await;
    }
}
