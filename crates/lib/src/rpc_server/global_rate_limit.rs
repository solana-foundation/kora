use futures_util::future::BoxFuture;
use std::{
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Instant,
};

use http::{Method, Request, Response, StatusCode};
use jsonrpsee::server::HttpBody as Body;
use tower::{Layer, Service, ServiceExt};

use crate::{
    constant::LIVENESS_PATH,
    metrics::HttpMetrics,
    rpc_server::{auth::RejectionReason, middleware_utils::build_response_with_graceful_error},
};

/// A custom token bucket primitive.
/// Built locally instead of reusing `tower::limit::RateLimit` because Tower
/// exerts backpressure (delays requests), whereas this endpoint requires
/// explicit HTTP 429 rejection.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
    refill_rate: f64,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self { capacity, tokens: capacity, last_refill: Instant::now(), refill_rate }
    }

    // Refills lazily based on elapsed time on every check rather than
    // requiring a dedicated background tick thread.
    pub fn consume(&mut self, amount: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }
}

// Shared token bucket enforcing a pre-auth limit across all incoming traffic.
#[derive(Clone)]
pub struct GlobalRateLimitLayer {
    bucket: Arc<Mutex<TokenBucket>>,
    record_metrics: bool,
    exempt_metrics_path: Option<String>,
}

impl GlobalRateLimitLayer {
    pub fn new(capacity: u64, record_metrics: bool, exempt_metrics_path: Option<String>) -> Self {
        let cap = capacity as f64;
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket::new(cap, cap))),
            record_metrics,
            exempt_metrics_path,
        }
    }
}

impl<S> Layer<S> for GlobalRateLimitLayer {
    type Service = GlobalRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GlobalRateLimitService {
            inner,
            bucket: self.bucket.clone(),
            record_metrics: self.record_metrics,
            exempt_metrics_path: self.exempt_metrics_path.clone(),
        }
    }
}

#[derive(Clone)]
pub struct GlobalRateLimitService<S> {
    inner: S,
    bucket: Arc<Mutex<TokenBucket>>,
    record_metrics: bool,
    exempt_metrics_path: Option<String>,
}

impl<S> Service<Request<Body>> for GlobalRateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Individual clones acquire readiness via oneshot in call().
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Health and metrics GET endpoints bypass shedding so monitoring remains operational under flood.
        let is_exempt = req.method() == Method::GET
            && (req.uri().path() == LIVENESS_PATH
                || self.exempt_metrics_path.as_deref() == Some(req.uri().path()));

        let allowed = is_exempt || {
            let mut bucket = self.bucket.lock().unwrap_or_else(|e| {
                log::warn!("Global rate limiter mutex was poisoned, recovering inner state");
                e.into_inner()
            });
            bucket.consume(1.0)
        };

        if allowed {
            let inner = self.inner.clone();
            return Box::pin(async move { inner.oneshot(req).await });
        }

        let record_metrics = self.record_metrics;
        Box::pin(async move {
            if record_metrics {
                HttpMetrics::get().record_global_rate_limit_rejection();
            }
            let mut res = build_response_with_graceful_error(
                None,
                StatusCode::TOO_MANY_REQUESTS,
                "Global rate limit exceeded",
            );
            res.extensions_mut().insert(RejectionReason::RateLimit);
            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };
    use tower::{ServiceBuilder, ServiceExt};

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(2.0, 2.0);

        assert!(bucket.consume(1.0));
        assert!(bucket.consume(1.0));
        assert!(!bucket.consume(1.0), "Bucket should be empty");

        std::thread::sleep(Duration::from_millis(550));
        assert!(bucket.consume(1.0), "Should have refilled 1 token after 0.5s");
        assert!(!bucket.consume(1.0), "Should not have refilled 2 tokens yet");
    }

    #[derive(Clone)]
    struct MockService {
        call_count: Arc<AtomicUsize>,
    }

    impl Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = std::convert::Infallible;
        type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(Response::builder().status(200).body(Body::empty()).unwrap()))
        }
    }

    #[tokio::test]
    async fn test_global_rate_limit_middleware() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockService { call_count: call_count.clone() };

        let layer = GlobalRateLimitLayer::new(1, false, None);
        let mut service = ServiceBuilder::new().layer(layer).service(mock_service);

        let req1 =
            Request::builder().method(Method::POST).uri("/test").body(Body::empty()).unwrap();
        let req2 =
            Request::builder().method(Method::POST).uri("/test").body(Body::empty()).unwrap();

        let res1 = service.ready().await.unwrap().call(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        let res2 = service.ready().await.unwrap().call(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res2.extensions().get::<RejectionReason>(), Some(&RejectionReason::RateLimit));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_global_rate_limit_liveness_bypass_only_for_get() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockService { call_count: call_count.clone() };

        let layer = GlobalRateLimitLayer::new(1, false, None);
        let mut service = ServiceBuilder::new().layer(layer).service(mock_service);

        // First POST to LIVENESS_PATH consumes the only token in the bucket.
        let post1 =
            Request::builder().method(Method::POST).uri(LIVENESS_PATH).body(Body::empty()).unwrap();
        let res1 = service.ready().await.unwrap().call(post1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second POST to LIVENESS_PATH must be rate-limited (429) because the bucket is now empty.
        let post2 =
            Request::builder().method(Method::POST).uri(LIVENESS_PATH).body(Body::empty()).unwrap();
        let res2 = service.ready().await.unwrap().call(post2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res2.extensions().get::<RejectionReason>(), Some(&RejectionReason::RateLimit));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // GET to LIVENESS_PATH must always pass even when the bucket is empty.
        let get_req =
            Request::builder().method(Method::GET).uri(LIVENESS_PATH).body(Body::empty()).unwrap();
        let res3 = service.ready().await.unwrap().call(get_req).await.unwrap();
        assert_eq!(res3.status(), StatusCode::OK);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_global_rate_limit_metrics_bypass_only_for_get() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock_service = MockService { call_count: call_count.clone() };

        let layer = GlobalRateLimitLayer::new(1, false, Some("/metrics".to_string()));
        let mut service = ServiceBuilder::new().layer(layer).service(mock_service);

        // First POST to /metrics consumes the only token in the bucket.
        let post1 =
            Request::builder().method(Method::POST).uri("/metrics").body(Body::empty()).unwrap();
        let res1 = service.ready().await.unwrap().call(post1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second POST to /metrics must be rate-limited (429) because the bucket is now empty.
        let post2 =
            Request::builder().method(Method::POST).uri("/metrics").body(Body::empty()).unwrap();
        let res2 = service.ready().await.unwrap().call(post2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res2.extensions().get::<RejectionReason>(), Some(&RejectionReason::RateLimit));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // GET to /metrics must always pass even when the bucket is empty.
        let get_req =
            Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();
        let res3 = service.ready().await.unwrap().call(get_req).await.unwrap();
        assert_eq!(res3.status(), StatusCode::OK);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }
}
