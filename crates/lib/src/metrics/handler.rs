use crate::rpc_server::middleware_utils::build_response_with_graceful_error;
use futures_util::future::BoxFuture;
use http::{Request, Response, StatusCode};
use jsonrpsee::server::HttpBody as Body;
use std::{
    collections::HashMap,
    task::{Context, Poll},
};
use tower::{Layer, Service, ServiceExt};

/// Layer that intercepts /metrics requests and returns Prometheus metrics directly
#[derive(Clone)]
pub struct MetricsHandlerLayer {
    endpoint: String,
}

impl MetricsHandlerLayer {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Default for MetricsHandlerLayer {
    fn default() -> Self {
        Self { endpoint: "/metrics".to_string() }
    }
}

impl<S> Layer<S> for MetricsHandlerLayer {
    type Service = MetricsHandlerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsHandlerService { inner, endpoint: self.endpoint.clone() }
    }
}

#[derive(Clone)]
pub struct MetricsHandlerService<S> {
    inner: S,
    endpoint: String,
}

impl<S> Service<Request<Body>> for MetricsHandlerService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Individual clones acquire readiness via oneshot in call().
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if req.method() == http::Method::GET && req.uri().path() == self.endpoint {
            Box::pin(async move {
                match crate::metrics::gather() {
                    Ok(metrics) => Ok(build_response_with_graceful_error(
                        Some(HashMap::from([(
                            "content-type".to_string(),
                            "text/plain; version=0.0.4".to_string(),
                        )])),
                        StatusCode::OK,
                        &metrics,
                    )),
                    Err(e) => {
                        log::error!("Failed to gather Prometheus metrics: {e}");
                        Ok(build_response_with_graceful_error(
                            None,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("Error gathering metrics: {e}"),
                        ))
                    }
                }
            })
        } else {
            let inner = self.inner.clone();
            Box::pin(async move { inner.oneshot(req).await })
        }
    }
}
