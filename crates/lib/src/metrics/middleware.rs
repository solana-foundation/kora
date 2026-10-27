use crate::{
    constant::LIVENESS_PATH,
    rpc_server::{
        auth::RejectionReason,
        middleware_utils::{extract_parts_and_body_bytes, get_jsonrpc_method},
    },
};
use http::{Request, Response};
use jsonrpsee::server::logger::Body;
use prometheus::{CounterVec, HistogramVec, Opts};
use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
    task::Poll,
    time::Instant,
};
use tower::{Layer, ServiceExt};

static HTTP_METRICS: OnceLock<HttpMetrics> = OnceLock::new();

const UNKNOWN_METHOD: &str = "unknown";
const ERROR_STATUS: &str = "error";

pub struct HttpMetrics {
    pub requests_total: CounterVec,
    pub request_duration_seconds: HistogramVec,
    pub http_rejections_total: CounterVec,
}

impl HttpMetrics {
    fn new() -> Self {
        let requests_total = CounterVec::new(
            Opts::new("http_requests_total", "Total number of HTTP requests").namespace("kora"),
            &["method", "status"],
        )
        .unwrap_or_else(|e| {
            log::error!("Failed to create http_requests_total metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });

        let request_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .namespace("kora")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["method"],
        )
        .unwrap_or_else(|e| {
            log::error!("Failed to create http_request_duration_seconds metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });

        let http_rejections_total = CounterVec::new(
            Opts::new("http_rejections_total", "Total number of rejected HTTP requests")
                .namespace("kora"),
            &["method", "reason"],
        )
        .unwrap_or_else(|e| {
            log::error!("Failed to create http_rejections_total metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });

        prometheus::register(Box::new(requests_total.clone())).unwrap_or_else(|e| {
            log::error!("Failed to register http_requests_total metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });
        prometheus::register(Box::new(request_duration_seconds.clone())).unwrap_or_else(|e| {
            log::error!("Failed to register http_request_duration_seconds metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });
        prometheus::register(Box::new(http_rejections_total.clone())).unwrap_or_else(|e| {
            log::error!("Failed to register http_rejections_total metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });

        Self { requests_total, request_duration_seconds, http_rejections_total }
    }

    pub fn get() -> &'static HttpMetrics {
        HTTP_METRICS.get_or_init(HttpMetrics::new)
    }

    pub fn record_global_rate_limit_rejection(&self) {
        let status = http::StatusCode::TOO_MANY_REQUESTS.as_u16().to_string();
        self.requests_total.with_label_values(&[UNKNOWN_METHOD, &status]).inc();
        self.http_rejections_total
            .with_label_values(&[UNKNOWN_METHOD, RejectionReason::RateLimit.as_str()])
            .inc();
    }
}
/// Tower layer for collecting HTTP metrics
#[derive(Clone)]
pub struct HttpMetricsLayer {
    allowed_methods: Arc<HashSet<String>>,
}

impl HttpMetricsLayer {
    pub fn new(allowed_methods: Arc<HashSet<String>>) -> Self {
        Self { allowed_methods }
    }
}

impl Default for HttpMetricsLayer {
    fn default() -> Self {
        Self::new(Arc::new(HashSet::new()))
    }
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetricsService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpMetricsService { inner: service, allowed_methods: self.allowed_methods.clone() }
    }
}

/// Tower service for collecting HTTP metrics
#[derive(Clone)]
pub struct HttpMetricsService<S> {
    inner: S,
    allowed_methods: Arc<HashSet<String>>,
}

impl<S> tower::Service<Request<Body>> for HttpMetricsService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Tower requires poll_ready to be called before call.
        // We delay polling inner here because we clone inner below;
        // cloning a Buffer yields an un-polled clone without a permit.
        // We use `oneshot` to poll the clone.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let start = Instant::now();
        let inner = self.inner.clone();
        let allowed_methods = self.allowed_methods.clone();

        Box::pin(async move {
            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;

            let is_liveness =
                parts.method == http::Method::GET && parts.uri.path() == LIVENESS_PATH;

            let mut method = if is_liveness {
                "liveness".to_string()
            } else {
                get_jsonrpc_method(&body_bytes).unwrap_or(UNKNOWN_METHOD.to_string())
            };

            if !is_liveness && !allowed_methods.contains(&method) {
                method = UNKNOWN_METHOD.to_string();
            }

            let new_body = Body::from(body_bytes);
            let new_request = Request::from_parts(parts, new_body);

            let result = inner.oneshot(new_request).await;

            let metrics = HttpMetrics::get();
            let duration = start.elapsed();

            match &result {
                Ok(response) => {
                    let status = response.status().as_u16().to_string();
                    metrics.requests_total.with_label_values(&[&method, &status]).inc();
                    metrics
                        .request_duration_seconds
                        .with_label_values(&[&method])
                        .observe(duration.as_secs_f64());
                    if let Some(reason) = response.extensions().get::<RejectionReason>() {
                        metrics
                            .http_rejections_total
                            .with_label_values(&[&method, reason.as_str()])
                            .inc();
                    }
                }
                Err(_) => {
                    metrics
                        .requests_total
                        .with_label_values(&[&method, &ERROR_STATUS.to_string()])
                        .inc();
                }
            }

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc_server::{
        auth::RejectionReason, recaptcha::RecaptchaLayer, recaptcha_util::RecaptchaConfig,
    };
    use http::{Method, StatusCode};
    use serial_test::serial;
    use std::{
        convert,
        future::Ready,
        task::{Context, Poll},
    };
    use tower::{Layer, Service, ServiceExt};

    #[derive(Clone)]
    struct MockService;

    impl tower::Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = convert::Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            std::future::ready(Ok(Response::builder().status(200).body(Body::empty()).unwrap()))
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_recaptcha_rejection_increments_http_rejections_total() {
        let recaptcha_config = RecaptchaConfig::new(
            "test-secret".to_string(),
            0.5,
            vec!["signTransaction".to_string()],
        );

        let mut allowed = HashSet::new();
        allowed.insert("signTransaction".to_string());

        let service = HttpMetricsLayer::new(Arc::new(allowed))
            .layer(RecaptchaLayer::new(recaptcha_config).layer(MockService));
        let mut service = tower::ServiceBuilder::new().service(service);

        let method_name = "signTransaction";
        let body = format!(r#"{{"jsonrpc":"2.0","method":"{}","id":1}}"#, method_name);

        let counter = &HttpMetrics::get().http_rejections_total;
        let before = counter
            .get_metric_with_label_values(&[method_name, RejectionReason::AuthFailure.as_str()])
            .map(|c| c.get())
            .unwrap_or(0.0);

        let request =
            Request::builder().method(Method::POST).uri("/").body(Body::from(body)).unwrap();

        let response = service.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let after = counter
            .get_metric_with_label_values(&[method_name, RejectionReason::AuthFailure.as_str()])
            .map(|c| c.get())
            .unwrap_or(0.0);

        assert_eq!(
            after - before,
            1.0,
            "http_rejections_total{{method=\"{}\",reason=\"{}\"}} should have incremented by 1",
            method_name,
            RejectionReason::AuthFailure.as_str(),
        );
    }
}
