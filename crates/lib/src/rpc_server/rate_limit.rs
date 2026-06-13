use crate::rpc_server::{
    auth::{ClientIdentity, RejectionReason},
    middleware_utils::build_response_with_graceful_error,
};
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};
use tower::{Layer, Service};

const UNAUTHENTICATED_IDENTITY: &str = "unauthenticated";

#[derive(Clone, Debug)]
pub struct TokenBucket {
    pub capacity: f64,
    pub tokens: f64,
    pub last_refill: Instant,
    pub refill_rate: f64, // tokens per second
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self { capacity, tokens: capacity, last_refill: Instant::now(), refill_rate }
    }

    pub fn consume(&mut self, amount: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens += elapsed * self.refill_rate;
        if self.tokens > self.capacity {
            self.tokens = self.capacity;
        }
        self.last_refill = now;

        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct IdentityRateLimitLayer {
    rate_limit: u64,
    state: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl IdentityRateLimitLayer {
    pub fn new(rate_limit: u64) -> Self {
        Self { rate_limit, state: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl<S> Layer<S> for IdentityRateLimitLayer {
    type Service = IdentityRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IdentityRateLimitService { inner, rate_limit: self.rate_limit, state: self.state.clone() }
    }
}

#[derive(Clone)]
pub struct IdentityRateLimitService<S> {
    inner: S,
    rate_limit: u64,
    state: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl<S> Service<Request<Body>> for IdentityRateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // rate_limit=0 disables per-identity limiting
        if self.rate_limit == 0 {
            return Box::pin(self.inner.call(request));
        }

        let identity = match request.extensions().get::<ClientIdentity>() {
            Some(id) => id.0.clone(),
            None => UNAUTHENTICATED_IDENTITY.to_string(),
        };

        let allowed = {
            let mut map = self.state.write();
            let bucket = map.entry(identity).or_insert_with(|| {
                TokenBucket::new(self.rate_limit as f64, self.rate_limit as f64)
            });
            bucket.consume(1.0)
        };

        if !allowed {
            let mut response =
                build_response_with_graceful_error(None, StatusCode::TOO_MANY_REQUESTS, "");
            response.extensions_mut().insert(RejectionReason::RateLimit);
            return Box::pin(async move { Ok(response) });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(request).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use std::future::Ready;
    use tower::{Service, ServiceExt};

    #[derive(Clone)]
    struct MockService;

    impl Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = std::convert::Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<Body>) -> Self::Future {
            std::future::ready(Ok(Response::new(Body::empty())))
        }
    }

    #[tokio::test]
    async fn test_identity_rate_limit_exceed_quota() {
        let layer = IdentityRateLimitLayer::new(2); // 2 requests per second
        let mut service = layer.layer(MockService);

        let mut req1 = Request::builder().body(Body::empty()).unwrap();
        req1.extensions_mut().insert(ClientIdentity("clientA".to_string()));
        let res1 = service.ready().await.unwrap().call(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        let mut req2 = Request::builder().body(Body::empty()).unwrap();
        req2.extensions_mut().insert(ClientIdentity("clientA".to_string()));
        let res2 = service.ready().await.unwrap().call(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);

        // Third request should be rate limited
        let mut req3 = Request::builder().body(Body::empty()).unwrap();
        req3.extensions_mut().insert(ClientIdentity("clientA".to_string()));
        let res3 = service.ready().await.unwrap().call(req3).await.unwrap();
        assert_eq!(res3.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res3.extensions().get::<RejectionReason>(), Some(&RejectionReason::RateLimit));
    }

    #[tokio::test]
    async fn test_independent_identities() {
        let layer = IdentityRateLimitLayer::new(1); // 1 request per second
        let mut service = layer.layer(MockService);

        // ClientA uses their 1 quota
        let mut req1 = Request::builder().body(Body::empty()).unwrap();
        req1.extensions_mut().insert(ClientIdentity("clientA".to_string()));
        let res1 = service.ready().await.unwrap().call(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        // ClientA is now rate limited
        let mut req2 = Request::builder().body(Body::empty()).unwrap();
        req2.extensions_mut().insert(ClientIdentity("clientA".to_string()));
        let res2 = service.ready().await.unwrap().call(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);

        // ClientB has their own independent quota
        let mut req3 = Request::builder().body(Body::empty()).unwrap();
        req3.extensions_mut().insert(ClientIdentity("clientB".to_string()));
        let res3 = service.ready().await.unwrap().call(req3).await.unwrap();
        assert_eq!(res3.status(), StatusCode::OK);
    }
}
