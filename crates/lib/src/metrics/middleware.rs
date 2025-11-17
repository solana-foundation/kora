use crate::rpc_server::middleware_utils::{extract_parts_and_body_bytes, get_jsonrpc_method};
use http::{Request, Response};
use jsonrpsee::server::logger::Body;
use prometheus::{CounterVec, HistogramVec, Opts};
use std::{sync::OnceLock, time::Instant};
use tower::Layer;

static HTTP_METRICS: OnceLock<HttpMetrics> = OnceLock::new();

const UNKNOWN_METHOD: &str = "unknown";
const ERROR_STATUS: &str = "error";

pub struct HttpMetrics {
    pub requests_total: CounterVec,
    pub request_duration_seconds: HistogramVec,
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

        prometheus::register(Box::new(requests_total.clone())).unwrap_or_else(|e| {
            log::error!("Failed to register http_requests_total metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });
        prometheus::register(Box::new(request_duration_seconds.clone())).unwrap_or_else(|e| {
            log::error!("Failed to register http_request_duration_seconds metric: {e:?}");
            panic!("Metrics initialization failed - cannot continue")
        });

        Self { requests_total, request_duration_seconds }
    }

    pub fn get() -> &'static HttpMetrics {
        HTTP_METRICS.get_or_init(HttpMetrics::new)
    }
}
/// Tower layer for collecting HTTP metrics
#[derive(Clone)]
pub struct HttpMetricsLayer;

impl HttpMetricsLayer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpMetricsLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetricsService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpMetricsService { inner: service }
    }
}

/// Tower service for collecting HTTP metrics
#[derive(Clone)]
pub struct HttpMetricsService<S> {
    inner: S,
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

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let start = Instant::now();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let (parts, body_bytes) = extract_parts_and_body_bytes(request).await;
            let method = get_jsonrpc_method(&body_bytes).unwrap_or(UNKNOWN_METHOD.to_string());

            // Reconstruct the request with the consumed body
            let new_body = Body::from(body_bytes);
            let new_request = Request::from_parts(parts, new_body);

            // Call the inner service
            let result = inner.call(new_request).await;

            // Record metrics
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
