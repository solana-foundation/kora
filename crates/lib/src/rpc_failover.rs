//! Multi-endpoint RPC failover.
//!
//! Implements [`RpcSender`] over an ordered endpoint list, so every RPC call is
//! retried against the next endpoint when the current one is unreachable or
//! answers 5xx. Wrapping the sender keeps existing `&RpcClient` call sites
//! unchanged.
//!
//! Each endpoint has a circuit breaker. A recovered endpoint is readmitted by a
//! single probe, and a retry never walks into an endpoint already failing.
//!
//! See <https://github.com/solana-foundation/kora/issues/661>.

use std::{
    sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use serde_json::Value;
use solana_client::{
    client_error::{reqwest::StatusCode, ClientError, ClientErrorKind, Result},
    rpc_request::RpcRequest,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_rpc_client::http_sender::HttpSender;

use crate::{
    constant::{
        RPC_CLIENT_TIMEOUT_SECS, RPC_FAILOVER_COOLDOWN_SECS, RPC_FAILOVER_FAILURE_THRESHOLD,
    },
    metrics::rpc_failover as failover_metrics,
    sanitize_error,
};

const COOLDOWN_MS: u64 = RPC_FAILOVER_COOLDOWN_SECS.saturating_mul(1000);

/// How long a half-open probe owns its slot before another request may take it
/// over. Longer than the HTTP timeout, but finite so a cancelled probe cannot
/// strand the endpoint.
const PROBE_TIMEOUT_MS: u64 = (RPC_CLIENT_TIMEOUT_SECS + 5) * 1000;

/// Circuit state of a single endpoint.
const CLOSED: u8 = 0;
/// Failing. New requests are turned away until `open_until_ms`.
const OPEN: u8 = 1;
/// Cooldown elapsed and one probe is in flight. Only the prober is admitted.
const HALF_OPEN: u8 = 2;

/// Milliseconds since the Unix epoch, saturating to 0 if the clock is before it.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// Why a particular call failed, and therefore whether another endpoint is worth
/// trying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureMode {
    /// The endpoint could not serve the request.
    Retryable(FailureReason),
    /// The provider is throttling this API key.
    RateLimited,
    /// The node answered, and the answer is final.
    Fatal,
}

/// Coarse classification of a retryable failure, used both in logs and as a
/// metric label so an operator can tell a provider outage from a network fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureReason {
    /// The endpoint answered 5xx.
    ServerError,
    /// The request never produced an HTTP response (DNS, TCP, TLS, timeout).
    Transport,
    /// A success status with a body that is not JSON, so something other than a
    /// Solana node produced it.
    MalformedResponse,
}

impl FailureReason {
    fn as_str(self) -> &'static str {
        match self {
            FailureReason::ServerError => "server_error",
            FailureReason::Transport => "transport",
            FailureReason::MalformedResponse => "malformed_response",
        }
    }
}

/// Decide how a failed RPC call should be handled.
///
/// Reads the typed error rather than its text, so a number in a message
/// (`insufficient lamports 5000`) cannot be mistaken for an HTTP status.
fn classify(error: &ClientError) -> FailureMode {
    match error.kind() {
        ClientErrorKind::Reqwest(source) => match source.status() {
            // 429 is a property of this API key, not the endpoint; `HttpSender`
            // has already retried it.
            Some(status) if status == StatusCode::TOO_MANY_REQUESTS => FailureMode::RateLimited,
            Some(status) if status.is_server_error() => {
                FailureMode::Retryable(FailureReason::ServerError)
            }
            // Other 4xx are well-formed refusals and will be identical elsewhere.
            Some(_) => FailureMode::Fatal,
            // No status means the request never got an HTTP response.
            None => FailureMode::Retryable(FailureReason::Transport),
        },
        ClientErrorKind::SerdeJson(_) => FailureMode::Retryable(FailureReason::MalformedResponse),
        ClientErrorKind::Io(_) => FailureMode::Retryable(FailureReason::Transport),
        // A JSON-RPC answer from a working node would be the same everywhere.
        _ => FailureMode::Fatal,
    }
}

/// One configured endpoint, its transport, and its circuit breaker.
struct Endpoint {
    sender: Box<dyn RpcSender + Send + Sync>,
    state: AtomicU8,
    /// Unix millis after which an `OPEN` breaker becomes probeable.
    open_until_ms: AtomicU64,
    /// Unix millis of the most recent retryable failure, or zero if there has been
    /// none. Used to decide whether a success is newer than the failures it would
    /// otherwise paper over.
    last_failure_at_ms: AtomicU64,
    consecutive_failures: AtomicU32,
}

impl Endpoint {
    fn new(sender: Box<dyn RpcSender + Send + Sync>) -> Self {
        Self {
            sender,
            state: AtomicU8::new(CLOSED),
            open_until_ms: AtomicU64::new(0),
            last_failure_at_ms: AtomicU64::new(0),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    fn state(&self) -> u8 {
        self.state.load(Ordering::Acquire)
    }

    fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    fn open_until_ms(&self) -> u64 {
        self.open_until_ms.load(Ordering::Relaxed)
    }

    fn last_failure_at_ms(&self) -> u64 {
        self.last_failure_at_ms.load(Ordering::Relaxed)
    }

    /// Whether new requests are currently being turned away. Only
    /// [`Endpoint::claim_probe`] can hand out a probe slot.
    fn is_turned_away(&self, now_ms: u64) -> bool {
        match self.state() {
            CLOSED => false,
            _ => now_ms < self.open_until_ms(),
        }
    }

    /// Take the probe slot, if available. The deadline CAS makes exactly one caller
    /// the prober, and lets a later request take over a probe that never reported
    /// back. `allow_cooling` is used only when every endpoint is cooling, so a total
    /// outage is still probed rather than failing outright.
    fn claim_probe(&self, now_ms: u64, allow_cooling: bool) -> bool {
        loop {
            if self.state() == CLOSED {
                return false;
            }

            let observed = self.open_until_ms();
            if !allow_cooling && now_ms < observed {
                return false;
            }

            let deadline = now_ms.saturating_add(PROBE_TIMEOUT_MS);
            if self
                .open_until_ms
                .compare_exchange(observed, deadline, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.state.store(HALF_OPEN, Ordering::Release);
                return true;
            }
        }
    }

    /// Record a retryable failure. The breaker opens only at
    /// [`RPC_FAILOVER_FAILURE_THRESHOLD`] consecutive failures; a failed probe
    /// reopens it immediately.
    fn record_failure(&self, now_ms: u64) {
        self.last_failure_at_ms.store(now_ms, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;

        if failures >= RPC_FAILOVER_FAILURE_THRESHOLD {
            self.open_until_ms.store(now_ms.saturating_add(COOLDOWN_MS), Ordering::Relaxed);
            self.state.store(OPEN, Ordering::Release);
        }
    }

    /// Record a success for an attempt that began at `attempt_started_ms`. A success
    /// only closes the breaker if nothing failed after the attempt started: the
    /// request timeout is longer than the cooldown, so a straggler would otherwise
    /// clear a breaker that newer failures opened.
    fn record_success(&self, attempt_started_ms: u64) {
        if self.last_failure_at_ms() > attempt_started_ms {
            return;
        }

        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.open_until_ms.store(0, Ordering::Relaxed);
        self.state.store(CLOSED, Ordering::Release);

        // A failure that landed between the guard above and this reset must win.
        // Otherwise a straggler success would erase the failure that just opened
        // the breaker and send the fleet back to a degraded endpoint. Keep the
        // count at the threshold so a failed probe still reopens immediately.
        if self.last_failure_at_ms() > attempt_started_ms {
            self.consecutive_failures.store(RPC_FAILOVER_FAILURE_THRESHOLD, Ordering::Relaxed);
            self.open_until_ms.store(now_ms().saturating_add(COOLDOWN_MS), Ordering::Relaxed);
            self.state.store(OPEN, Ordering::Release);
        }
    }
}

/// An endpoint this request may consider.
struct Candidate {
    index: usize,
    needs_probe: bool,
    /// The total-outage path already owns the slot.
    probe_claimed: bool,
}

/// A retryable failure held until another endpoint is actually attempted, so a
/// skipped candidate is not counted as a failover.
struct PendingFailure {
    endpoint_index: usize,
    reason: &'static str,
    consecutive_failures: u32,
    is_probe: bool,
    error: ClientError,
}

/// The error returned when every endpoint is being probed by another request.
fn no_endpoint_available() -> ClientError {
    ClientError::from(ClientErrorKind::Custom(
        "no RPC endpoint is currently available: every configured endpoint is being probed"
            .to_string(),
    ))
}

/// An [`RpcSender`] that retries each request against successive endpoints when the
/// current one is unreachable or answers 5xx.
pub struct FailoverRpcSender {
    endpoints: Vec<Endpoint>,
}

impl FailoverRpcSender {
    /// Build a sender over `urls`, in preference order.
    ///
    /// # Panics
    /// Panics if `urls` is empty. Use
    /// [`get_rpc_client_for_endpoints`](crate::rpc::get_rpc_client_for_endpoints)
    /// for the validating entry point.
    pub fn new(urls: Vec<String>) -> Self {
        assert!(!urls.is_empty(), "at least one RPC endpoint is required");

        let endpoints = urls
            .into_iter()
            .map(|url| {
                Endpoint::new(Box::new(HttpSender::new_with_timeout(
                    url,
                    Duration::from_secs(RPC_CLIENT_TIMEOUT_SECS),
                )))
            })
            .collect();

        Self { endpoints }
    }

    /// Consecutive retryable failures recorded against the endpoint at `index`.
    #[cfg(test)]
    pub fn consecutive_failures_at(&self, index: usize) -> u32 {
        self.endpoints[index].consecutive_failures()
    }

    /// Circuit state of the endpoint at `index`, as `closed`, `open` or
    /// `half_open`.
    #[cfg(test)]
    pub fn state_at(&self, index: usize) -> &'static str {
        match self.endpoints[index].state() {
            CLOSED => "closed",
            OPEN => "open",
            _ => "half_open",
        }
    }

    /// Endpoints this request may consider, best first. Side-effect free: claims
    /// happen only when an attempt starts, and an expired probe slot is included so
    /// an abandoned probe cannot hide a recovered endpoint.
    fn candidates(&self, now_ms: u64) -> Vec<Candidate> {
        self.endpoints
            .iter()
            .enumerate()
            .filter_map(|(index, endpoint)| match endpoint.state() {
                CLOSED => Some(Candidate { index, needs_probe: false, probe_claimed: false }),
                OPEN | HALF_OPEN if now_ms >= endpoint.open_until_ms() => {
                    Some(Candidate { index, needs_probe: true, probe_claimed: false })
                }
                _ => None,
            })
            .collect()
    }

    /// Take the probe slot for the endpoint whose cooldown expires first. Reached
    /// only when every endpoint is open, so a total outage is still probed. Returns
    /// `None` when another request already holds every probe.
    fn claim_soonest_reopening(&self, now_ms: u64) -> Option<usize> {
        let soonest = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, endpoint)| endpoint.state() == OPEN)
            .min_by_key(|(_, endpoint)| endpoint.open_until_ms())
            .map(|(index, _)| index)?;

        // Deliberately ignore the remaining cooldown: if every endpoint is open,
        // one request probing beats every request failing.
        self.endpoints[soonest].claim_probe(now_ms, true).then_some(soonest)
    }

    /// Index of the endpoint a new request would prefer, for reporting only.
    fn preferred_endpoint_at(&self, now_ms: u64) -> usize {
        self.endpoints.iter().position(|endpoint| !endpoint.is_turned_away(now_ms)).unwrap_or(0)
    }

    /// Index of the endpoint a new request would prefer, using the current clock.
    fn preferred_endpoint(&self) -> usize {
        self.preferred_endpoint_at(now_ms())
    }
}

#[async_trait]
impl RpcSender for FailoverRpcSender {
    async fn send(&self, request: RpcRequest, params: Value) -> Result<Value> {
        let mut candidates = self.candidates(now_ms());

        if candidates.is_empty() {
            match self.claim_soonest_reopening(now_ms()) {
                Some(index) => {
                    candidates = vec![Candidate { index, needs_probe: true, probe_claimed: true }]
                }
                None => return Err(no_endpoint_available()),
            }
        }

        let mut pending: Option<PendingFailure> = None;

        for candidate in candidates {
            let now = now_ms();
            let endpoint = &self.endpoints[candidate.index];

            // The list was built before the earlier attempts ran, so re-check it:
            // an endpoint that looked healthy can have opened while this request
            // waited on a slow one.
            if !candidate.probe_claimed {
                if endpoint.is_turned_away(now) {
                    continue;
                }

                // Claiming happens here rather than when the list was built, so a
                // request that never reaches this endpoint leaves no claim behind.
                if candidate.needs_probe && !endpoint.claim_probe(now, false) {
                    // Another request is already probing this endpoint.
                    continue;
                }
            }

            // This attempt is real, so the previous failure did cause a failover.
            // Counting it earlier would overstate failovers when the next candidate
            // turns out to be skipped without an attempt.
            if let Some(failed) = pending.take() {
                failover_metrics::record_failover(failed.endpoint_index, failed.reason);
                log::warn!(
                    "RPC endpoint {} failed ({}), {} consecutive failures, \
                     trying the next endpoint: {}",
                    failed.endpoint_index,
                    failed.reason,
                    failed.consecutive_failures,
                    sanitize_error!(failed.error),
                );
            }

            let is_probe = candidate.needs_probe;
            let attempt_started_ms = now;

            match endpoint.sender.send(request, params.clone()).await {
                Ok(value) => {
                    endpoint.record_success(attempt_started_ms);
                    failover_metrics::set_active_endpoint(candidate.index);
                    return Ok(value);
                }
                Err(error) => match classify(&error) {
                    FailureMode::Fatal => {
                        // The endpoint answered, so it is healthy. Not counting this
                        // against its breaker is what keeps a legitimate
                        // "account not found" from tripping the circuit.
                        endpoint.record_success(attempt_started_ms);
                        return Err(error);
                    }
                    FailureMode::RateLimited => {
                        endpoint.record_success(attempt_started_ms);
                        log::debug!(
                            "RPC endpoint {} is rate limiting, not failing over",
                            candidate.index
                        );
                        return Err(error);
                    }
                    FailureMode::Retryable(reason) => {
                        // Read the clock here, not when the request started: a
                        // request timeout is longer than the cooldown, so reusing
                        // the start time would set a deadline already in the past.
                        endpoint.record_failure(now_ms());
                        pending = Some(PendingFailure {
                            endpoint_index: candidate.index,
                            reason: reason.as_str(),
                            consecutive_failures: endpoint.consecutive_failures(),
                            is_probe,
                            error,
                        });
                    }
                },
            }
        }

        match pending {
            Some(failed) => {
                // A terminal failure is not a failover, so it is neither counted as
                // one nor logged as one.
                log::warn!(
                    "RPC endpoint {} failed ({}) with no endpoint left to try{}: {}",
                    failed.endpoint_index,
                    failed.reason,
                    if failed.is_probe { ", half-open probe failed" } else { "" },
                    sanitize_error!(failed.error),
                );
                Err(failed.error)
            }
            None => Err(no_endpoint_available()),
        }
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        // One logical call can touch several endpoints, so totals are summed
        // across all of them rather than read from a single sender.
        self.endpoints.iter().fold(RpcTransportStats::default(), |mut total, endpoint| {
            let stats = endpoint.sender.get_transport_stats();
            total.request_count += stats.request_count;
            total.elapsed_time += stats.elapsed_time;
            total.rate_limited_time += stats.rate_limited_time;
            total
        })
    }

    fn url(&self) -> String {
        self.endpoints[self.preferred_endpoint()].sender.url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constant::RPC_FAILOVER_COOLDOWN_SECS, metrics::rpc_failover as failover_metrics,
        rpc::get_rpc_client_for_endpoints,
    };
    use serde_json::json;
    use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
    use std::sync::{atomic::AtomicU64, Arc};

    const COOLDOWN_MS: u64 = RPC_FAILOVER_COOLDOWN_SECS * 1000;
    const CLOSED_AT: u64 = 1_000_000;

    /// A JSON-RPC success body for `getLatestBlockhash`.
    const BLOCKHASH_RESPONSE: &str = r#"{"jsonrpc":"2.0","result":{"context":{"apiVersion":"2.2.1","slot":1},"value":{"blockhash":"11111111111111111111111111111111","lastValidBlockHeight":100}},"id":1}"#;

    /// A port nothing listens on, so connecting to it is refused outright.
    const CLOSED_PORT_URL: &str = "http://127.0.0.1:1";

    /// A JSON-RPC error body, which is what a working node returns for things like
    /// a missing account.
    fn rpc_error(message: &str) -> ClientError {
        RpcError::RpcResponseError {
            code: -32602,
            message: message.to_string(),
            data: RpcResponseErrorData::Empty,
        }
        .into()
    }

    /// A transport-level error, which is what an unreachable endpoint produces.
    fn transport_error(message: &str) -> ClientError {
        ClientError::from(ClientErrorKind::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            message.to_string(),
        )))
    }

    /// A sender that answers with a canned result and counts requests. The error
    /// case takes a factory because [`ClientError`] is not `Clone`.
    struct ScriptedSender {
        url: String,
        hits: Arc<AtomicU64>,
        respond: Box<dyn Fn() -> Result<Value> + Send + Sync>,
    }

    /// A scripted endpoint plus a live request counter.
    type Counted = (ScriptedSender, Arc<AtomicU64>);

    impl ScriptedSender {
        fn always_succeeding(url: &str, value: Value) -> Counted {
            let hits = Arc::new(AtomicU64::new(0));
            let sender = Self {
                url: url.to_string(),
                hits: hits.clone(),
                respond: Box::new(move || Ok(value.clone())),
            };

            (sender, hits)
        }

        fn always_failing(
            url: &str,
            error: impl Fn() -> ClientError + Send + Sync + 'static,
        ) -> Counted {
            let hits = Arc::new(AtomicU64::new(0));
            let sender = Self {
                url: url.to_string(),
                hits: hits.clone(),
                respond: Box::new(move || Err(error())),
            };

            (sender, hits)
        }
    }

    #[async_trait]
    impl RpcSender for ScriptedSender {
        async fn send(&self, _request: RpcRequest, _params: Value) -> Result<Value> {
            self.hits.fetch_add(1, Ordering::Relaxed);
            (self.respond)()
        }

        fn get_transport_stats(&self) -> RpcTransportStats {
            RpcTransportStats::default()
        }

        fn url(&self) -> String {
            self.url.clone()
        }
    }

    /// A sender that never answers, standing in for an endpoint that hangs.
    struct HangingSender;

    #[async_trait]
    impl RpcSender for HangingSender {
        async fn send(&self, _request: RpcRequest, _params: Value) -> Result<Value> {
            std::future::pending::<Result<Value>>().await
        }

        fn get_transport_stats(&self) -> RpcTransportStats {
            RpcTransportStats::default()
        }

        fn url(&self) -> String {
            "http://hanging".to_string()
        }
    }

    fn hits(counter: &Arc<AtomicU64>) -> u64 {
        counter.load(Ordering::Relaxed)
    }

    /// A failover sender over the given scripted endpoints.
    fn failover_over(senders: Vec<Box<dyn RpcSender + Send + Sync>>) -> FailoverRpcSender {
        assert!(!senders.is_empty());
        FailoverRpcSender { endpoints: senders.into_iter().map(Endpoint::new).collect() }
    }

    /// Two endpoints where only the second answers.
    fn primary_down_secondary_up() -> (FailoverRpcSender, Arc<AtomicU64>, Arc<AtomicU64>) {
        let (primary, primary_hits) =
            ScriptedSender::always_failing("http://primary", || transport_error("primary down"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("from-secondary"));

        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        (sender, primary_hits, secondary_hits)
    }

    /// Drive the endpoint at `index` to the failure threshold.
    fn open_breaker(sender: &FailoverRpcSender, index: usize, at_ms: u64) {
        for _ in 0..RPC_FAILOVER_FAILURE_THRESHOLD {
            sender.endpoints[index].record_failure(at_ms);
        }
    }

    #[test]
    #[should_panic(expected = "at least one RPC endpoint is required")]
    fn test_new_rejects_empty_endpoints() {
        FailoverRpcSender::new(vec![]);
    }

    #[tokio::test]
    async fn test_healthy_primary_answers() {
        let (primary, primary_hits) =
            ScriptedSender::always_succeeding("http://primary", json!("ok"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_failing("http://secondary", || transport_error("unused"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        let response = sender
            .send(RpcRequest::GetLatestBlockhash, json!([]))
            .await
            .expect("primary should answer");

        assert_eq!(response, json!("ok"));
        assert_eq!(hits(&primary_hits), 1);
        assert_eq!(hits(&secondary_hits), 0, "a healthy primary must not need the fallback");
        assert_eq!(sender.state_at(0), "closed");
    }

    #[tokio::test]
    async fn test_transport_failure_fails_over_to_next_endpoint() {
        let (sender, primary_hits, secondary_hits) = primary_down_secondary_up();

        let response = sender
            .send(RpcRequest::GetLatestBlockhash, json!([]))
            .await
            .expect("secondary should answer");

        assert_eq!(response, json!("from-secondary"));
        assert_eq!(hits(&primary_hits), 1, "the primary must be tried first");
        assert_eq!(hits(&secondary_hits), 1);
        assert_eq!(sender.consecutive_failures_at(0), 1);
    }

    #[tokio::test]
    async fn test_final_answer_is_not_retried_elsewhere() {
        let (primary, primary_hits) =
            ScriptedSender::always_failing("http://primary", || rpc_error("Account not found"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("x"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        let error =
            sender.send(RpcRequest::GetAccountInfo, json!([])).await.expect_err("should fail");

        assert!(error.to_string().contains("Account not found"));
        assert_eq!(hits(&primary_hits), 1);
        assert_eq!(hits(&secondary_hits), 0, "a real answer must not be retried");
        assert_eq!(sender.consecutive_failures_at(0), 0, "a real answer is not a fault");
        assert_eq!(sender.state_at(0), "closed", "a real answer must not trip the breaker");
    }

    #[tokio::test]
    async fn test_malformed_body_fails_over() {
        let (primary, _) = ScriptedSender::always_failing("http://primary", || {
            serde_json::from_str::<Value>("<html>502</html>").expect_err("not json").into()
        });
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("ok"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        assert_eq!(
            sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.unwrap(),
            json!("ok")
        );
        assert_eq!(hits(&secondary_hits), 1);
    }

    #[tokio::test]
    async fn test_every_endpoint_failing_returns_the_last_error() {
        let (first, first_hits) =
            ScriptedSender::always_failing("http://first", || transport_error("first down"));
        let (second, second_hits) =
            ScriptedSender::always_failing("http://second", || transport_error("second down"));
        let sender = failover_over(vec![Box::new(first), Box::new(second)]);

        let error =
            sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect_err("fails");

        assert!(error.to_string().contains("second down"), "got: {error}");
        assert_eq!(hits(&first_hits), 1);
        assert_eq!(hits(&second_hits), 1, "every configured endpoint must be tried once");
    }

    #[tokio::test]
    async fn test_endpoint_keeps_serving_below_the_threshold() {
        let (sender, primary_hits, secondary_hits) = primary_down_secondary_up();

        for _ in 1..RPC_FAILOVER_FAILURE_THRESHOLD {
            sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");
        }

        assert_eq!(sender.consecutive_failures_at(0), RPC_FAILOVER_FAILURE_THRESHOLD - 1);
        assert_eq!(sender.state_at(0), "closed", "a couple of blips must not open the breaker");
        assert_eq!(sender.preferred_endpoint_at(CLOSED_AT), 0, "the primary is still preferred");
        assert_eq!(hits(&primary_hits) as u32, RPC_FAILOVER_FAILURE_THRESHOLD - 1);
        assert_eq!(hits(&secondary_hits) as u32, RPC_FAILOVER_FAILURE_THRESHOLD - 1);
    }

    #[tokio::test]
    async fn test_endpoint_opens_once_the_threshold_is_reached() {
        let (sender, primary_hits, secondary_hits) = primary_down_secondary_up();

        for _ in 0..RPC_FAILOVER_FAILURE_THRESHOLD {
            sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");
        }

        assert_eq!(sender.state_at(0), "open");
        assert_eq!(sender.preferred_endpoint_at(now_ms()), 1, "traffic moves to the fallback");

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");
        assert_eq!(
            hits(&primary_hits) as u32,
            RPC_FAILOVER_FAILURE_THRESHOLD,
            "the open primary must not be tried again"
        );
        assert_eq!(hits(&secondary_hits) as u32, RPC_FAILOVER_FAILURE_THRESHOLD + 1);
    }

    #[tokio::test]
    async fn test_retry_skips_an_endpoint_that_is_already_cooling() {
        let (primary, primary_hits) =
            ScriptedSender::always_failing("http://primary", || transport_error("primary down"));
        let (middle, middle_hits) =
            ScriptedSender::always_failing("http://middle", || transport_error("middle down"));
        let (last, last_hits) = ScriptedSender::always_succeeding("http://last", json!("ok"));

        let sender = failover_over(vec![Box::new(primary), Box::new(middle), Box::new(last)]);
        open_breaker(&sender, 1, now_ms());

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");

        assert_eq!(hits(&primary_hits), 1);
        assert_eq!(hits(&middle_hits), 0, "a cooling endpoint must not be retried into");
        assert_eq!(hits(&last_hits), 1, "the request must reach the healthy endpoint");
    }

    #[test]
    fn test_cooldown_admits_exactly_one_probe() {
        let sender = failover_over(vec![
            Box::new(ScriptedSender::always_succeeding("http://primary", json!(null)).0),
            Box::new(ScriptedSender::always_succeeding("http://secondary", json!(null)).0),
        ]);

        let failed_at = CLOSED_AT;
        open_breaker(&sender, 0, failed_at);
        assert_eq!(sender.state_at(0), "open");

        let after_cooldown = failed_at + COOLDOWN_MS;

        let first = sender.candidates(after_cooldown);
        assert_eq!(first.len(), 2, "the probe joins the fallback");
        assert!(first[0].needs_probe, "the recovered primary is probed first");

        // Building the list must not claim anything, so a second request still sees
        // the probe as available.
        assert_eq!(sender.state_at(0), "open");

        assert!(
            sender.endpoints[0].claim_probe(after_cooldown, false),
            "the first request wins the probe"
        );
        assert_eq!(sender.state_at(0), "half_open");
        assert!(
            !sender.endpoints[0].claim_probe(after_cooldown, false),
            "only one request may probe"
        );

        let second = sender.candidates(after_cooldown);
        assert_eq!(second.len(), 1, "everyone else keeps using the fallback");
        assert_eq!(second[0].index, 1);
        assert!(!second[0].needs_probe);
    }

    #[tokio::test]
    async fn test_success_on_the_primary_leaves_no_claim_on_the_fallback() {
        // Regression: a request that lists a probe-able fallback but is answered by
        // the primary must not leave that fallback stuck half-open, which would
        // exclude it from all future traffic even after it recovered.
        let (primary, _) = ScriptedSender::always_succeeding("http://primary", json!("primary"));
        let (secondary, _) =
            ScriptedSender::always_succeeding("http://secondary", json!("secondary"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        for index in 0..2 {
            open_breaker(&sender, index, CLOSED_AT);
            sender.endpoints[index].open_until_ms.store(CLOSED_AT, Ordering::Relaxed);
        }

        let listed = sender.candidates(CLOSED_AT);
        assert_eq!(listed.len(), 2, "both endpoints look available");
        assert!(listed.iter().all(|candidate| candidate.needs_probe));

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("primary answers");

        assert_eq!(sender.state_at(0), "closed");
        assert_eq!(
            sender.state_at(1),
            "open",
            "the untouched fallback must not be left claimed as half-open"
        );
    }

    #[tokio::test]
    async fn test_request_errors_instead_of_panicking_when_every_probe_is_held() {
        // Regression: this used to reach `last_error.expect(..)` with no candidate
        // tried and panic inside an RPC handler.
        let (primary, _) =
            ScriptedSender::always_failing("http://primary", || transport_error("down"));
        let (secondary, _) =
            ScriptedSender::always_failing("http://secondary", || transport_error("down"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        for index in 0..2 {
            open_breaker(&sender, index, CLOSED_AT);
            assert!(sender.endpoints[index].claim_probe(now_ms(), false));
            assert_eq!(sender.state_at(index), "half_open");
        }

        let error = sender
            .send(RpcRequest::GetLatestBlockhash, json!([]))
            .await
            .expect_err("there is nothing to try");

        assert!(
            error.to_string().contains("no RPC endpoint is currently available"),
            "expected a clear error, got: {error}"
        );
    }

    #[tokio::test]
    async fn test_a_cancelled_probe_is_reclaimed_after_its_slot_expires() {
        // Regression: a probe whose request is dropped (client disconnect, task
        // abort) used to leave the endpoint in HALF_OPEN with no way to claim a
        // new probe, taking a recovered fallback out of rotation for good.
        let sender = Arc::new(failover_over(vec![Box::new(HangingSender)]));

        sender.endpoints[0].state.store(OPEN, Ordering::Release);
        sender.endpoints[0].open_until_ms.store(0, Ordering::Relaxed);

        let request = {
            let sender = sender.clone();
            tokio::spawn(
                async move { sender.send(RpcRequest::GetLatestBlockhash, json!([])).await },
            )
        };

        for _ in 0..1_000 {
            if sender.state_at(0) == "half_open" {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(sender.state_at(0), "half_open", "the request should hold the probe");

        request.abort();
        let _ = request.await;

        assert_eq!(sender.state_at(0), "half_open");
        assert!(
            sender.endpoints[0].open_until_ms() > now_ms(),
            "the abandoned probe still owns its slot"
        );
        assert!(sender.candidates(now_ms()).is_empty(), "a live probe is not handed out twice");

        sender.endpoints[0].open_until_ms.store(now_ms().saturating_sub(1), Ordering::Relaxed);
        let candidates = sender.candidates(now_ms());
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].needs_probe);
        assert!(sender.endpoints[0].claim_probe(now_ms(), false));
    }

    #[tokio::test]
    async fn test_successful_probe_closes_the_breaker() {
        let (primary, _) = ScriptedSender::always_succeeding("http://primary", json!("back"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("ok"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        open_breaker(&sender, 0, CLOSED_AT);
        sender.endpoints[0].open_until_ms.store(CLOSED_AT, Ordering::Relaxed);

        let response = sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("probe");

        assert_eq!(response, json!("back"), "the probe must be answered by the primary");
        assert_eq!(sender.state_at(0), "closed");
        assert_eq!(sender.consecutive_failures_at(0), 0);
        assert_eq!(hits(&secondary_hits), 0, "a successful probe needs no fallback");
        assert_eq!(sender.preferred_endpoint_at(CLOSED_AT), 0, "the primary is preferred again");
    }

    #[test]
    fn test_a_straggler_success_does_not_close_a_breaker_opened_after_it_started() {
        // Regression: the request timeout is longer than the cooldown, so a response
        // already in flight when the breaker opened must not paper over it.
        let sender = failover_over(vec![Box::new(
            ScriptedSender::always_succeeding("http://primary", json!(null)).0,
        )]);

        let attempt_started_ms = now_ms().saturating_sub(1_000);
        open_breaker(&sender, 0, now_ms());
        assert_eq!(sender.state_at(0), "open");

        sender.endpoints[0].record_success(attempt_started_ms);

        assert_eq!(
            sender.state_at(0),
            "open",
            "a success older than the failures must not clear them"
        );
        assert!(sender.consecutive_failures_at(0) >= RPC_FAILOVER_FAILURE_THRESHOLD);
    }

    #[test]
    fn test_a_success_with_no_newer_failures_closes_the_breaker() {
        let sender = failover_over(vec![Box::new(
            ScriptedSender::always_succeeding("http://primary", json!(null)).0,
        )]);

        open_breaker(&sender, 0, now_ms());
        assert_eq!(sender.state_at(0), "open");

        // A probe that starts after the breaker opened is fresh evidence.
        sender.endpoints[0].record_success(now_ms());

        assert_eq!(sender.state_at(0), "closed");
        assert_eq!(sender.consecutive_failures_at(0), 0);
    }

    #[tokio::test]
    async fn test_failed_probe_reopens_the_breaker_and_restarts_the_cooldown() {
        let (primary, _) =
            ScriptedSender::always_failing("http://primary", || transport_error("still down"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("ok"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        open_breaker(&sender, 0, CLOSED_AT);
        sender.endpoints[0].open_until_ms.store(CLOSED_AT, Ordering::Relaxed);

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");

        assert_eq!(sender.state_at(0), "open", "a failed probe must reopen the breaker");
        assert!(sender.endpoints[0].open_until_ms() >= CLOSED_AT + COOLDOWN_MS);
        assert_eq!(hits(&secondary_hits), 1);
    }

    #[test]
    fn test_all_cooling_falls_back_to_a_single_probe() {
        let sender = failover_over(vec![
            Box::new(ScriptedSender::always_succeeding("http://primary", json!(null)).0),
            Box::new(ScriptedSender::always_succeeding("http://secondary", json!(null)).0),
        ]);

        open_breaker(&sender, 0, CLOSED_AT);
        open_breaker(&sender, 1, CLOSED_AT + COOLDOWN_MS / 2);

        assert!(sender.candidates(CLOSED_AT).is_empty(), "both endpoints are still cooling");

        let claimed = sender.claim_soonest_reopening(CLOSED_AT);

        assert_eq!(
            claimed,
            Some(0),
            "one request probes the endpoint whose cooldown expires first"
        );
        assert_eq!(sender.state_at(0), "half_open");
        assert_eq!(
            sender.claim_soonest_reopening(CLOSED_AT),
            Some(1),
            "the next request takes the other endpoint rather than piling onto the first"
        );
        assert_eq!(
            sender.claim_soonest_reopening(CLOSED_AT),
            None,
            "once every probe is held there is nothing left to claim"
        );
    }

    #[tokio::test]
    async fn test_success_clears_the_failure_count() {
        let (mut sender, _, _) = primary_down_secondary_up();

        for _ in 0..RPC_FAILOVER_FAILURE_THRESHOLD {
            sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");
        }
        assert_eq!(sender.state_at(0), "open");

        sender.endpoints[0].sender =
            Box::new(ScriptedSender::always_succeeding("http://primary", json!("ok")).0);
        sender.endpoints[0].open_until_ms.store(0, Ordering::Relaxed);

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("probe succeeds");

        assert_eq!(sender.state_at(0), "closed");
        assert_eq!(sender.consecutive_failures_at(0), 0);
    }

    #[test]
    fn test_classify_maps_transport_and_malformed_errors_to_retryable() {
        assert_eq!(
            classify(&transport_error("unreachable")),
            FailureMode::Retryable(FailureReason::Transport)
        );

        let malformed: ClientError =
            serde_json::from_str::<Value>("not json").expect_err("not json").into();
        assert_eq!(classify(&malformed), FailureMode::Retryable(FailureReason::MalformedResponse));
    }

    #[test]
    fn test_classify_treats_rpc_answers_as_fatal() {
        assert_eq!(classify(&rpc_error("Account not found")), FailureMode::Fatal);
        assert_eq!(classify(&rpc_error("Blockhash not found")), FailureMode::Fatal);
    }

    // `RPC_FAILOVERS_TOTAL` is process-global, so each metrics test gives its own
    // endpoint a unique index and holds the earlier ones open.

    /// A sender whose first `reserved` endpoints are open and cooling, leaving index
    /// `reserved` as the first real candidate.
    fn sender_behind_reserved_index(
        reserved: usize,
        tail: Vec<Box<dyn RpcSender + Send + Sync>>,
    ) -> FailoverRpcSender {
        let mut senders: Vec<Box<dyn RpcSender + Send + Sync>> =
            Vec::with_capacity(reserved + tail.len());
        for _ in 0..reserved {
            senders
                .push(Box::new(ScriptedSender::always_succeeding("http://cooling", json!(null)).0));
        }
        senders.extend(tail);

        let sender = failover_over(senders);
        let far_future = now_ms().saturating_add(COOLDOWN_MS.saturating_mul(10));
        for index in 0..reserved {
            open_breaker(&sender, index, now_ms());
            sender.endpoints[index].open_until_ms.store(far_future, Ordering::Relaxed);
        }

        sender
    }

    // Distinct label indexes keep the process-global metric counters isolated.
    const RESERVED_FOR_SWITCH: usize = 90;
    const RESERVED_FOR_TERMINAL: usize = 93;
    const RESERVED_FOR_OPENED: usize = 96;

    #[tokio::test]
    async fn test_a_switch_is_counted_once() {
        let (failing, _) =
            ScriptedSender::always_failing("http://failing", || transport_error("down"));
        let (working, _) = ScriptedSender::always_succeeding("http://working", json!("ok"));
        let sender = sender_behind_reserved_index(
            RESERVED_FOR_SWITCH,
            vec![Box::new(failing), Box::new(working)],
        );

        let before = failover_metrics::counter_value(RESERVED_FOR_SWITCH, "transport");
        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("fails over");

        assert_eq!(
            failover_metrics::counter_value(RESERVED_FOR_SWITCH, "transport") - before,
            1,
            "one switch to the next endpoint is one failover"
        );
    }

    #[tokio::test]
    async fn test_a_terminal_failure_is_not_counted_as_a_failover() {
        let (only, _) = ScriptedSender::always_failing("http://only", || transport_error("down"));
        let sender = sender_behind_reserved_index(RESERVED_FOR_TERMINAL, vec![Box::new(only)]);

        let before = failover_metrics::counter_value(RESERVED_FOR_TERMINAL, "transport");
        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect_err("no endpoint left");

        assert_eq!(
            failover_metrics::counter_value(RESERVED_FOR_TERMINAL, "transport"),
            before,
            "nothing was failed over to, so nothing is counted"
        );
    }

    #[tokio::test]
    async fn test_an_endpoint_that_opens_mid_request_is_not_retried_into_or_counted() {
        // Regression: the candidate list is built before the first attempt, so an
        // endpoint can open while this request waits. Retrying into it would add
        // another timeout, and counting the switch would report a failover that
        // never happened.
        let sender = Arc::new_cyclic(|weak| {
            let weak = weak.clone();
            let (failing, _) = ScriptedSender::always_failing("http://failing", move || {
                let sender = weak.upgrade().expect("the sender is alive for the whole call");
                open_breaker(&sender, RESERVED_FOR_OPENED + 1, now_ms());
                transport_error("first down")
            });
            let (working, _) =
                ScriptedSender::always_succeeding("http://working", json!("should not be used"));

            sender_behind_reserved_index(
                RESERVED_FOR_OPENED,
                vec![Box::new(failing), Box::new(working)],
            )
        });

        let before = failover_metrics::counter_value(RESERVED_FOR_OPENED, "transport");
        let error = sender
            .send(RpcRequest::GetLatestBlockhash, json!([]))
            .await
            .expect_err("the opened endpoint must not answer");

        assert!(error.to_string().contains("first down"), "got: {error}");
        assert_eq!(
            failover_metrics::counter_value(RESERVED_FOR_OPENED, "transport"),
            before,
            "no endpoint was actually switched to"
        );
    }

    // `reqwest::Error` cannot be constructed in-process, so real responses are
    // needed to cover 5xx/429 classification.

    #[tokio::test]
    async fn test_every_server_error_status_fails_over() {
        for status in [500u16, 501, 502, 503, 504] {
            let mut failing = mockito::Server::new_async().await;
            let mut healthy = mockito::Server::new_async().await;

            let failing_mock = failing
                .mock("POST", "/")
                .with_status(status as usize)
                .with_body(r#"{"error":"upstream"}"#)
                .create_async()
                .await;
            let healthy_mock = healthy
                .mock("POST", "/")
                .with_status(200)
                .with_body(BLOCKHASH_RESPONSE)
                .expect(1)
                .create_async()
                .await;

            let client = get_rpc_client_for_endpoints(vec![failing.url(), healthy.url()])
                .expect("client should build");

            client
                .get_latest_blockhash()
                .await
                .unwrap_or_else(|e| panic!("status {status} should have failed over: {e}"));

            failing_mock.assert_async().await;
            healthy_mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn test_real_http_429_surfaces_without_failing_over() {
        let mut throttled = mockito::Server::new_async().await;
        let mut spare = mockito::Server::new_async().await;

        // `HttpSender` retries a 429 five times before surfacing it.
        let throttled_mock = throttled
            .mock("POST", "/")
            .with_status(429)
            .with_header("retry-after", "0")
            .expect(6)
            .create_async()
            .await;
        let spare_mock = spare
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(0)
            .create_async()
            .await;

        let client = get_rpc_client_for_endpoints(vec![throttled.url(), spare.url()])
            .expect("client should build");

        let error = client.get_latest_blockhash().await.expect_err("a rate limit should surface");

        assert!(error.to_string().contains("429"), "unexpected error: {error}");
        throttled_mock.assert_async().await;
        spare_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_real_http_400_is_not_retried_elsewhere() {
        let mut refusing = mockito::Server::new_async().await;
        let mut spare = mockito::Server::new_async().await;

        let refusing_mock = refusing
            .mock("POST", "/")
            .with_status(400)
            .with_body("bad request")
            .expect(1)
            .create_async()
            .await;
        let spare_mock = spare
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(0)
            .create_async()
            .await;

        let client = get_rpc_client_for_endpoints(vec![refusing.url(), spare.url()])
            .expect("client should build");

        assert!(client.get_latest_blockhash().await.is_err());

        refusing_mock.assert_async().await;
        spare_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_real_connection_refused_fails_over() {
        let mut healthy = mockito::Server::new_async().await;
        let healthy_mock = healthy
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(1)
            .create_async()
            .await;

        let client = get_rpc_client_for_endpoints(vec![CLOSED_PORT_URL.to_string(), healthy.url()])
            .expect("client should build");

        client.get_latest_blockhash().await.expect("should fail over");
        healthy_mock.assert_async().await;
    }
}
