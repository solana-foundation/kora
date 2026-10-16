use futures_util::future::BoxFuture;
use std::{
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Instant,
};

use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
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
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
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

// Wraps a single shared bucket to enforce a global limit across all clients.
// Sits after CORS and before auth, so 429 responses still carry CORS headers.
#[derive(Clone)]
pub struct GlobalRateLimitLayer {
    bucket: Arc<Mutex<TokenBucket>>,
    record_metrics: bool,
}

impl GlobalRateLimitLayer {
    pub fn new(capacity: u64, record_metrics: bool) -> Self {
        let cap = capacity as f64;
        Self { bucket: Arc::new(Mutex::new(TokenBucket::new(cap, cap))), record_metrics }
    }
}

impl<S> Layer<S> for GlobalRateLimitLayer {
    type Service = GlobalRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GlobalRateLimitService {
            inner,
            bucket: self.bucket.clone(),
            record_metrics: self.record_metrics,
        }
    }
}

#[derive(Clone)]
pub struct GlobalRateLimitService<S> {
    inner: S,
    bucket: Arc<Mutex<TokenBucket>>,
    record_metrics: bool,
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
        // Tower requires poll_ready to be called before call.
        // We delay polling inner here because we clone inner below;
        // cloning a Buffer yields an un-polled clone without a permit.
        // We use `oneshot` to poll the clone.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Exempt liveness endpoints so the server remains observable when under heavy load.
        // Metrics bypass is handled by the outer MetricsHandlerLayer.
        if req.uri().path() == LIVENESS_PATH {
            let inner = self.inner.clone();
            return Box::pin(async move { inner.oneshot(req).await });
        }

        let allowed = {
            // Recover instead of panicking to prevent a single panicked thread from poisoning the global rate limiter.
            let mut bucket = self.bucket.lock().unwrap_or_else(|e| e.into_inner());
            bucket.consume(1.0)
        };

        if allowed {
            let inner = self.inner.clone();
            Box::pin(async move { inner.oneshot(req).await })
        } else {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
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

        let layer = GlobalRateLimitLayer::new(1, false);
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

        // Ensure inner service was NOT called
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
