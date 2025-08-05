use futures_util::future::BoxFuture;
use http::{Request, Response, StatusCode};
use jsonrpsee::server::logger::Body;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Layer that intercepts /metrics requests and returns Prometheus metrics directly
#[derive(Clone, Default)]
pub struct MetricsHandlerLayer;

impl<S> Layer<S> for MetricsHandlerLayer {
    type Service = MetricsHandlerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsHandlerService { inner }
    }
}

#[derive(Clone)]
pub struct MetricsHandlerService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MetricsHandlerService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Check if this is a metrics request
        if req.uri().path() == "/metrics" && req.method() == http::Method::GET {
            // Return metrics directly
            Box::pin(async move {
                match crate::gather() {
                    Ok(metrics) => {
                        let response = Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "text/plain; version=0.0.4")
                            .body(Body::from(metrics))
                            .unwrap();
                        Ok(response)
                    }
                    Err(e) => {
                        let response = Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from(format!("Error gathering metrics: {e}")))
                            .unwrap();
                        Ok(response)
                    }
                }
            })
        } else {
            // Pass through to inner service
            let mut inner = self.inner.clone();
            Box::pin(async move { inner.call(req).await })
        }
    }
}
