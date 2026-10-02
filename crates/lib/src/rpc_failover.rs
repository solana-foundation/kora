//! Multi-endpoint RPC failover.
//!
//! Implements [`RpcSender`] over an ordered list of endpoints so that every RPC
//! call Kora makes is retried against the next endpoint when the current one is
//! unreachable or answers 5xx. Failing over at the sender rather than at the
//! [`RpcClient`] keeps this transparent to the existing `&RpcClient` call sites in
//! the crate, so fee estimation, account lookups, blockhash fetches,
//! lookup-table resolution, bundle simulation and the balance tracker are all
//! covered without changing a signature.
//!
//! Each endpoint runs a circuit breaker, and every request builds its candidate
//! list from the breakers rather than only consulting the first one. That matters
//! in two directions: a recovered endpoint is readmitted by a single probe
//! instead of by every concurrent request at once, and a retry never walks into
//! an endpoint already known to be failing.
//!
//! Failover is driven by failures on the calls Kora actually makes. A passing
//! `getHealth` is not evidence that the account-data methods work, so there is no
//! health prober here.
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
/// Classification reads the typed error rather than its text, so unrelated numbers
/// in an error message (`insufficient lamports 5000`) cannot be mistaken for an
/// HTTP status. The Solana sender turns a non-2xx response into a `reqwest::Error`
/// carrying the status, and a JSON-RPC `error` object into
/// `RpcError::RpcResponseError`; the two are handled very differently.
fn classify(error: &ClientError) -> FailureMode {
    match error.kind() {
        ClientErrorKind::Reqwest(source) => match source.status() {
            // `HttpSender` has already retried 429 five times over, honouring any
            // `Retry-After` header on each, before surfacing the error. A rate
            // limit is a property of this API key rather than of the endpoint, so
            // moving the same traffic to another provider would not help.
            Some(status) if status == StatusCode::TOO_MANY_REQUESTS => FailureMode::RateLimited,
            Some(status) if status.is_server_error() => {
                FailureMode::Retryable(FailureReason::ServerError)
            }
            // Any other 4xx is a well-formed refusal (revoked key, malformed
            // request) and will be refused identically everywhere else.
            Some(_) => FailureMode::Fatal,
            // No status means the request never reached an HTTP response. A refused
            // connection, a failed TLS handshake and a read timeout are all
            // indistinguishable here and all mean the same thing to us.
            None => FailureMode::Retryable(FailureReason::Transport),
        },
        ClientErrorKind::SerdeJson(_) => FailureMode::Retryable(FailureReason::MalformedResponse),
        ClientErrorKind::Io(_) => FailureMode::Retryable(FailureReason::Transport),
        // Everything else is a JSON-RPC answer such as "account not found" or a
        // failed simulation. Those come from a working node, so another endpoint
        // would only produce the same answer more slowly.
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

    /// Whether new requests are currently being turned away.
    ///
    /// An open breaker becomes probeable once its cooldown elapses, but the probe
    /// itself is handed out by [`FailoverRpcSender::candidates_at`], not here:
    /// reading the state must never admit anybody.
    fn is_turned_away(&self, now_ms: u64) -> bool {
        match self.state() {
            CLOSED => false,
            OPEN => now_ms < self.open_until_ms(),
            _ => true,
        }
    }

    /// Take the half-open probe slot for this endpoint, if it is still available.
    ///
    /// Exactly one caller wins the compare-and-swap, so a provider that recovered in
    /// name only cannot make every concurrent request wait out its timeout. The
    /// losers are expected to move on to another endpoint.
    fn claim_probe(&self) -> bool {
        self.state.compare_exchange(OPEN, HALF_OPEN, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }

    /// Record a retryable failure.
    ///
    /// The breaker only opens once the failure count reaches
    /// [`RPC_FAILOVER_FAILURE_THRESHOLD`], so one flaky response does not move
    /// traffic off an otherwise healthy provider. A failed probe reopens the
    /// breaker immediately, since a probe only runs once the endpoint has already
    /// been given a chance to recover.
    fn record_failure(&self, now_ms: u64) {
        self.last_failure_at_ms.store(now_ms, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;

        if failures >= RPC_FAILOVER_FAILURE_THRESHOLD {
            self.open_until_ms.store(now_ms.saturating_add(COOLDOWN_MS), Ordering::Relaxed);
            self.state.store(OPEN, Ordering::Release);
        } else {
            let _ =
                self.state.compare_exchange(HALF_OPEN, OPEN, Ordering::AcqRel, Ordering::Acquire);
        }
    }

    /// Record a success for an attempt that began at `attempt_started_ms`.
    ///
    /// A response that was already in flight when the breaker opened says nothing
    /// about the endpoint's condition now, and the request timeout is longer than
    /// the cooldown, so acting on it would let one straggler clear a breaker that
    /// newer failures opened and send the whole fleet back to a slow provider. A
    /// success is therefore only allowed to close the breaker if nothing failed
    /// after the attempt started.
    fn record_success(&self, attempt_started_ms: u64) {
        if self.last_failure_at_ms() > attempt_started_ms {
            return;
        }

        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.open_until_ms.store(0, Ordering::Relaxed);
        self.state.store(CLOSED, Ordering::Release);
    }
}

/// An endpoint this request may consider, and whether it has to claim the
/// half-open probe slot before it can be used.
struct Candidate {
    index: usize,
    needs_probe: bool,
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
    ///
    /// Panics if `urls` is empty. Callers that need the empty case handled should
    /// use [`get_rpc_client_for_endpoints`](crate::rpc::get_rpc_client_for_endpoints),
    /// which validates and returns a ready client.
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

    /// Number of configured endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Consecutive retryable failures recorded against the endpoint at `index`.
    pub fn consecutive_failures_at(&self, index: usize) -> u32 {
        self.endpoints[index].consecutive_failures()
    }

    /// Circuit state of the endpoint at `index`, as `closed`, `open` or
    /// `half_open`.
    pub fn state_at(&self, index: usize) -> &'static str {
        match self.endpoints[index].state() {
            CLOSED => "closed",
            OPEN => "open",
            _ => "half_open",
        }
    }

    /// Endpoints this request may consider, best first.
    ///
    /// Deliberately free of side effects. Claiming the half-open probe slot happens
    /// when an attempt actually starts, so a request that succeeds on its first
    /// candidate cannot leave an unused claim behind on a fallback it never tried.
    fn candidates(&self, now_ms: u64) -> Vec<Candidate> {
        self.endpoints
            .iter()
            .enumerate()
            .filter_map(|(index, endpoint)| match endpoint.state() {
                CLOSED => Some(Candidate { index, needs_probe: false }),
                OPEN if now_ms >= endpoint.open_until_ms() => {
                    Some(Candidate { index, needs_probe: true })
                }
                _ => None,
            })
            .collect()
    }

    /// Take the probe slot for the endpoint whose cooldown expires first.
    ///
    /// Reached only when every endpoint is open with nothing recovered, so that a
    /// total outage is still probed rather than failing outright. Returns `None`
    /// when some other request already holds every probe, which is the case where
    /// this request has nothing useful to do.
    fn claim_soonest_reopening(&self, now_ms: u64) -> Option<usize> {
        let soonest = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, endpoint)| endpoint.state() == OPEN)
            .min_by_key(|(_, endpoint)| endpoint.open_until_ms())
            .map(|(index, _)| index)?;

        if self.endpoints[soonest].claim_probe() {
            self.endpoints[soonest].open_until_ms.store(now_ms, Ordering::Relaxed);
            Some(soonest)
        } else {
            None
        }
    }

    /// Index of the endpoint a new request would prefer, without claiming anything.
    ///
    /// For reporting the active endpoint rather than for routing.
    pub fn preferred_endpoint_at(&self, now_ms: u64) -> usize {
        self.endpoints.iter().position(|endpoint| !endpoint.is_turned_away(now_ms)).unwrap_or(0)
    }

    /// Index of the endpoint a new request would prefer, using the current clock.
    pub fn preferred_endpoint(&self) -> usize {
        self.preferred_endpoint_at(now_ms())
    }
}

#[async_trait]
impl RpcSender for FailoverRpcSender {
    async fn send(&self, request: RpcRequest, params: Value) -> Result<Value> {
        let mut candidates = self.candidates(now_ms());

        if candidates.is_empty() {
            match self.claim_soonest_reopening(now_ms()) {
                Some(index) => candidates = vec![Candidate { index, needs_probe: false }],
                None => return Err(no_endpoint_available()),
            }
        }

        let mut last_error: Option<ClientError> = None;

        for (position, candidate) in candidates.iter().enumerate() {
            let endpoint = &self.endpoints[candidate.index];

            // Claiming happens here rather than when the list was built, so a
            // request that never reaches this endpoint leaves no claim behind.
            if candidate.needs_probe && !endpoint.claim_probe() {
                // Another request is already probing this endpoint.
                continue;
            }

            let is_probe = candidate.needs_probe;
            let attempt_started_ms = now_ms();

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

                        // A terminal failure is not a failover, so it is neither
                        // counted as one nor logged as one.
                        if position + 1 < candidates.len() {
                            failover_metrics::record_failover(candidate.index, reason.as_str());
                            log::warn!(
                                "RPC endpoint {} failed ({}), {} consecutive failures, \
                                 trying the next endpoint: {}",
                                candidate.index,
                                reason.as_str(),
                                endpoint.consecutive_failures(),
                                sanitize_error!(error),
                            );
                        } else {
                            log::warn!(
                                "RPC endpoint {} failed ({}) with no endpoint left to try{}: {}",
                                candidate.index,
                                reason.as_str(),
                                if is_probe { ", half-open probe failed" } else { "" },
                                sanitize_error!(error),
                            );
                        }

                        last_error = Some(error);
                    }
                },
            }
        }

        Err(last_error.unwrap_or_else(no_endpoint_available))
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
    use prometheus::TextEncoder;
    use serde_json::json;
    use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
    use std::sync::{atomic::AtomicU64, Arc};

    const COOLDOWN_MS: u64 = RPC_FAILOVER_COOLDOWN_SECS * 1000;
    const CLOSED_AT: u64 = 1_000_000;

    /// A JSON-RPC success body for `getLatestBlockhash`.
    const BLOCKHASH_RESPONSE: &str = r#"{"jsonrpc":"2.0","result":{"context":{"apiVersion":"2.2.1","slot":1},"value":{"blockhash":"11111111111111111111111111111111","lastValidBlockHeight":100}},"id":1}"#;

    /// A port nothing listens on, so connecting to it is refused outright.
    const CLOSED_PORT_URL: &str = "http://127.0.0.1:1";

    /// A hostname that must never resolve. `.invalid` is reserved by RFC 2606
    /// precisely so it cannot be registered, which makes this a DNS failure
    /// rather than a slow machine.
    const UNRESOLVABLE_HOST_URL: &str = "http://kora-failover-probe.invalid";

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

    /// A sender that answers with a canned result and counts the requests it
    /// received, so tests can assert exactly which endpoint served a call.
    ///
    /// The error case takes a factory rather than a value because [`ClientError`]
    /// is not `Clone`, and an `Fn` closure cannot hand out an owned error twice.
    struct ScriptedSender {
        url: String,
        hits: Arc<AtomicU64>,
        respond: Box<dyn Fn() -> Result<Value> + Send + Sync>,
    }

    /// A scripted endpoint plus a live handle on the requests it received.
    ///
    /// The counter is shared rather than read off the sender because the sender is
    /// moved into the failover sender and the tests still assert on it afterwards.
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

    /// Drive the endpoint at `index` up to the failure threshold, which is what
    /// opens its breaker.
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

    #[test]
    fn test_new_counts_configured_endpoints() {
        let sender = FailoverRpcSender::new(vec![
            "http://localhost:8899".into(),
            "http://localhost:8900".into(),
        ]);

        assert_eq!(sender.endpoint_count(), 2);
    }

    // Routing
    // ------

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

    // Breaker
    // ------

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
        // Middle endpoint is open, so a request that fails on the primary must go
        // straight to the third rather than waiting on the middle one.
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

        assert!(sender.endpoints[0].claim_probe(), "the first request wins the probe");
        assert_eq!(sender.state_at(0), "half_open");
        assert!(!sender.endpoints[0].claim_probe(), "only one request may probe");

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

        // Both endpoints have recovered, so both are listed as probe candidates.
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

        // Another request holds the probe on every endpoint.
        for index in 0..2 {
            open_breaker(&sender, index, CLOSED_AT);
            assert!(sender.endpoints[index].claim_probe());
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
    async fn test_successful_probe_closes_the_breaker() {
        let (primary, _) = ScriptedSender::always_succeeding("http://primary", json!("back"));
        let (secondary, secondary_hits) =
            ScriptedSender::always_succeeding("http://secondary", json!("ok"));
        let sender = failover_over(vec![Box::new(primary), Box::new(secondary)]);

        open_breaker(&sender, 0, CLOSED_AT);
        // Pretend the cooldown has elapsed by rewriting the deadline.
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

        // The secondary is what has been answering, so let the primary recover and
        // serve once.
        sender.endpoints[0].sender =
            Box::new(ScriptedSender::always_succeeding("http://primary", json!("ok")).0);
        sender.endpoints[0].open_until_ms.store(0, Ordering::Relaxed);

        sender.send(RpcRequest::GetLatestBlockhash, json!([])).await.expect("probe succeeds");

        assert_eq!(sender.state_at(0), "closed");
        assert_eq!(sender.consecutive_failures_at(0), 0);
    }

    // Classification
    // ------

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

    #[test]
    fn test_failure_reason_labels_are_distinct() {
        let labels = [
            FailureReason::ServerError.as_str(),
            FailureReason::Transport.as_str(),
            FailureReason::MalformedResponse.as_str(),
        ];
        let unique: std::collections::HashSet<_> = labels.iter().collect();

        assert_eq!(unique.len(), labels.len(), "metric labels must be distinguishable");
    }

    // Metrics
    // ------
    //
    // `RPC_FAILOVERS_TOTAL` is process-global and these tests run in parallel, so a
    // counter read here can be moved by any other test between the read and the
    // assertion. The counter is labelled by an endpoint's position in its list, so
    // the only way to own a label exclusively is to give each test a sender whose
    // interesting endpoint sits at its own index, with everything before it held
    // open so it is skipped.

    /// A sender whose first `reserved` endpoints are open and still cooling, so they
    /// are never tried, leaving index `reserved` as the first real candidate.
    ///
    /// Deadlines are set against the real clock, because that is what `send` reads;
    /// a synthetic timestamp in the past looks long expired.
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

    /// Endpoint index owned by the switch-counting test.
    const RESERVED_FOR_SWITCH: usize = 90;
    /// Endpoint index owned by the terminal-failure test.
    const RESERVED_FOR_TERMINAL: usize = 93;

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

    #[test]
    fn test_metrics_render() {
        failover_metrics::record_failover(RESERVED_FOR_SWITCH, "server_error");
        failover_metrics::set_active_endpoint(1);

        let rendered = TextEncoder::new()
            .encode_to_string(&prometheus::gather())
            .expect("metrics should encode");

        assert!(rendered.contains("rpc_failovers_total"));
        assert!(rendered.contains("rpc_active_endpoint_index"));
    }

    // Real HTTP
    // ------
    //
    // These drive a real `HttpSender` against real servers. They are the only way
    // to cover the classification of a real 5xx or 429 response: `reqwest::Error`
    // has no public constructor and no `From<StatusCode>` conversion, so a
    // status-carrying error cannot be fabricated in-process.

    #[tokio::test]
    async fn test_real_http_500_fails_over_to_a_healthy_endpoint() {
        let mut broken = mockito::Server::new_async().await;
        let mut healthy = mockito::Server::new_async().await;

        let broken_mock = broken
            .mock("POST", "/")
            .with_status(500)
            .with_body(r#"{"error":"upstream unavailable"}"#)
            .create_async()
            .await;
        let healthy_mock = healthy
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(1)
            .create_async()
            .await;

        let client = get_rpc_client_for_endpoints(vec![broken.url(), healthy.url()])
            .expect("client should build");

        client.get_latest_blockhash().await.expect("should fail over");

        broken_mock.assert_async().await;
        healthy_mock.assert_async().await;
    }

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

    #[tokio::test]
    async fn test_dns_failure_fails_over() {
        let mut healthy = mockito::Server::new_async().await;
        let healthy_mock = healthy
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(1)
            .create_async()
            .await;

        let client =
            get_rpc_client_for_endpoints(vec![UNRESOLVABLE_HOST_URL.to_string(), healthy.url()])
                .expect("client should build");

        client.get_latest_blockhash().await.expect("an unresolvable host must fail over");
        healthy_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_tls_failure_fails_over() {
        let plaintext = mockito::Server::new_async().await;
        let mut healthy = mockito::Server::new_async().await;

        let healthy_mock = healthy
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(1)
            .create_async()
            .await;

        let https_url = plaintext.url().replacen("http://", "https://", 1);
        let client = get_rpc_client_for_endpoints(vec![https_url, healthy.url()])
            .expect("client should build");

        client.get_latest_blockhash().await.expect("a failed handshake must fail over");
        healthy_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_single_endpoint_client_serves_directly() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(BLOCKHASH_RESPONSE)
            .expect(1)
            .create_async()
            .await;

        let client = get_rpc_client_for_endpoints(vec![server.url()]).expect("client should build");

        client.get_latest_blockhash().await.expect("single endpoint should answer");
        mock.assert_async().await;
    }
}
