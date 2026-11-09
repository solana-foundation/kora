use std::{sync::LazyLock, time::Duration};

use async_trait::async_trait;
use reqwest::{header::HeaderValue, redirect::Policy, Client, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use url::{Host, Url};

use crate::{
    config::{Config, ExternalDecisionConfig},
    constant::MAX_EXTERNAL_DECISION_TIMEOUT_MS,
    error::KoraError,
    sanitize_error,
    transaction::{VersionedTransactionOps, VersionedTransactionResolved},
};

use super::{PluginExecutionContext, TransactionPlugin};

const MAX_DENY_REASON_CHARS: usize = 200;
// The shortest hex run redaction treats as a secret (`sanitize_error!`).
const KEY_HEX_CHARS: usize = 32;
// A decision is a few bytes; a provider's own evidence alongside it fits well under this.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

// Redirects are not followed, so the transaction and the token only reach the configured URL. A
// loopback endpoint is reached directly: a proxy (HTTP_PROXY or ALL_PROXY from the environment)
// would otherwise receive the plain-HTTP request, token included, and answer in its place.
fn build_client(builder: ClientBuilder, loopback: bool) -> Client {
    let builder = builder.redirect(Policy::none());
    let builder = if loopback { builder.no_proxy() } else { builder };
    builder.build().expect("Failed to build reqwest client")
}

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| build_client(Client::builder(), false));
static LOOPBACK_CLIENT: LazyLock<Client> = LazyLock::new(|| build_client(Client::builder(), true));

#[derive(Serialize)]
struct DecisionRequest {
    transaction: String,
    fee_payer: String,
    method: &'static str,
}

#[derive(Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
enum Decision {
    Allow,
    Deny { reason: Option<String> },
}

fn client_for(url: &Url) -> &'static Client {
    if is_loopback(url) {
        &LOOPBACK_CLIENT
    } else {
        &HTTP_CLIENT
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// What config validation reports about the token as resolved: one that cannot be read or sent.
fn token_problem(token: &Result<Option<String>, String>) -> Option<String> {
    match token {
        Ok(token) => token_error(token.as_deref()),
        Err(error) => Some(format!("ExternalDecision plugin: {error}")),
    }
}

/// Why a token cannot be sent, if it cannot (a trailing newline from a mounted secret, say).
fn token_error(token: Option<&str>) -> Option<String> {
    let token = token?;
    HeaderValue::from_str(&format!("Bearer {token}")).err().map(|_| {
        format!(
            "ExternalDecision plugin: {} is not a valid HTTP header value",
            ExternalDecisionConfig::TOKEN_ENV
        )
    })
}

// Control characters, and every character that does not show (Unicode's default-ignorable code
// points: direction marks and overrides, zero-width characters, variation selectors, fillers, tag
// characters) or hides the text it delimits (interlinear annotation marks). Where the client shows
// a reason, they can reorder or hide it, or carry text a model reads and a person does not.
fn is_hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}' | '\u{180B}'..='\u{180F}' | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}' | '\u{FFA0}' | '\u{FFF0}'..='\u{FFFB}'
            | '\u{1BCA0}'..='\u{1BCA3}' | '\u{1D173}'..='\u{1D17A}' | '\u{E0000}'..='\u{E0FFF}')
}

// Line breaks and tabs, which `is_hidden` also counts as control characters.
fn is_break(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

// Line breaks and other spaces become a space (a line or paragraph separator starts a new line
// wherever the reason is shown), except a break inside a hex run as long as a key, which is
// dropped: a key wrapped over lines (as `xxd -p` prints it) stays one run, so redaction still sees
// it. Hidden characters are dropped, and the cap applies to what the client receives, once secrets
// are redacted: a cut before it could leave part of a key unredacted.
fn deny_reason(reason: &str) -> String {
    let visible: Vec<char> =
        reason.chars().filter(|&c| c.is_whitespace() || !is_hidden(c)).collect();
    // For each position: the hex digits in a row from it, and the run that follows its breaks.
    let mut hex_from = vec![0usize; visible.len() + 1];
    let mut hex_after_breaks = vec![0usize; visible.len() + 1];
    for (i, &c) in visible.iter().enumerate().rev() {
        hex_from[i] = if c.is_ascii_hexdigit() { hex_from[i + 1] + 1 } else { 0 };
        hex_after_breaks[i] = if is_break(c) { hex_after_breaks[i + 1] } else { hex_from[i] };
    }
    let mut shown = String::with_capacity(reason.len());
    let mut hex_before = 0usize;
    for (i, &c) in visible.iter().enumerate() {
        let after = hex_after_breaks[i + 1];
        if is_break(c) && hex_before > 0 && after > 0 && hex_before + after >= KEY_HEX_CHARS {
            continue;
        }
        let c = if c.is_whitespace() { ' ' } else { c };
        hex_before = if c.is_ascii_hexdigit() { hex_before + 1 } else { 0 };
        shown.push(c);
    }
    sanitize_error!(shown).chars().take(MAX_DENY_REASON_CHARS).collect()
}

/// Asks an operator-run HTTP endpoint whether the fee payer may sign a transaction. Like the other
/// plugins, it runs after `validate_transaction` and before the payment, fee and transfer-hook
/// checks (in a bundle, after the transfer-hook check), so the endpoint can be asked about a
/// transaction Kora then refuses. The endpoint
/// receives the transaction as the client submitted it, the fee payer, and the RPC method. Only
/// an HTTP 200 answer of `{"decision": "allow"}` lets the transaction through; a deny, a timeout,
/// a transport error, any other status, or any other body refuses it.
///
/// Use case: a risk or policy service that catches fee payer abuse the structural checks miss,
/// without tying Kora to a particular provider.
pub(super) struct ExternalDecisionPlugin;

impl ExternalDecisionPlugin {
    /// The decision, asked with the token as resolved: a token that cannot be read asks nothing.
    async fn decide(
        config: &ExternalDecisionConfig,
        body: &DecisionRequest,
        token: Result<Option<String>, String>,
    ) -> Result<Decision, String> {
        let token = token?;
        Self::request_decision(config, body, token.as_deref()).await
    }

    async fn request_decision(
        config: &ExternalDecisionConfig,
        body: &DecisionRequest,
        token: Option<&str>,
    ) -> Result<Decision, String> {
        let url = Url::parse(&config.url).map_err(|_| "the url is not a valid URL".to_string())?;
        let mut request =
            client_for(&url).post(url).timeout(Duration::from_millis(config.timeout_ms)).json(body);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }

        // Errors are logged without their url, which may carry a credential.
        let response =
            request.send().await.map_err(|e| format!("request failed: {}", e.without_url()))?;
        let status = response.status();
        if status != StatusCode::OK {
            return Err(format!("endpoint returned HTTP {}", status.as_u16()));
        }

        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("failed to read response body: {}", e.without_url()))?
        {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(format!("response larger than {MAX_RESPONSE_BYTES} bytes"));
            }
            bytes.extend_from_slice(&chunk);
        }
        // An object only: serde also reads a tagged enum from an array such as ["allow"].
        if bytes.trim_ascii_start().first() != Some(&b'{') {
            return Err("unrecognized response: not a JSON object".to_string());
        }
        // serde's message quotes the body (an unknown decision, for one): only its position is
        // logged, so the endpoint cannot write a line of its own into Kora's log.
        serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "unrecognized response: {:?} error at line {} column {}",
                e.classify(),
                e.line(),
                e.column()
            )
        })
    }
}

#[async_trait]
impl TransactionPlugin for ExternalDecisionPlugin {
    async fn validate(
        &self,
        transaction: &VersionedTransactionResolved,
        config: &Config,
        _rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let Some(decision_config) = config.kora.plugins.external_decision.as_ref() else {
            return Err(KoraError::ConfigError(
                "ExternalDecision plugin is enabled without [kora.plugins.external_decision]"
                    .to_string(),
            ));
        };

        let body = DecisionRequest {
            transaction: transaction.encode_b64_transaction()?,
            fee_payer: fee_payer.to_string(),
            method: context.method_name(),
        };

        let decision =
            Self::decide(decision_config, &body, ExternalDecisionConfig::resolved_token()).await;
        match decision {
            Ok(Decision::Allow) => Ok(()),
            Ok(Decision::Deny { reason }) => {
                let denied = match reason.as_deref().map(deny_reason).filter(|r| !r.is_empty()) {
                    Some(reason) => format!("denied by the decision endpoint: {reason}"),
                    None => "denied by the decision endpoint".to_string(),
                };
                Err(KoraError::InvalidTransaction(format!(
                    "ExternalDecision plugin: transaction {denied} (context: {})",
                    context.method_name()
                )))
            }
            Err(detail) => {
                log::error!(
                    "ExternalDecision plugin: no decision for {}: {}",
                    context.method_name(),
                    sanitize_error!(detail)
                );
                Err(KoraError::InternalServerError(format!(
                    "ExternalDecision plugin: no valid decision from the decision endpoint \
                     (context: {})",
                    context.method_name()
                )))
            }
        }
    }

    fn validate_config(&self, config: &Config) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();

        let Some(decision_config) = config.kora.plugins.external_decision.as_ref() else {
            errors.push(
                "ExternalDecision plugin requires [kora.plugins.external_decision] with a url"
                    .to_string(),
            );
            return (errors, vec![]);
        };

        // The url itself is never echoed: it may carry a credential in its path or query.
        match Url::parse(&decision_config.url) {
            Ok(url) => {
                if url.scheme() != "https" && !(url.scheme() == "http" && is_loopback(&url)) {
                    errors.push(
                        "ExternalDecision plugin url must use https (http is accepted only for \
                         a loopback host)"
                            .to_string(),
                    );
                }
                if !url.username().is_empty() || url.password().is_some() {
                    errors.push(format!(
                        "ExternalDecision plugin url must not embed credentials; set {} instead",
                        ExternalDecisionConfig::TOKEN_ENV
                    ));
                }
            }
            Err(_) => errors.push("ExternalDecision plugin url is not a valid URL".to_string()),
        }

        errors.extend(token_problem(&ExternalDecisionConfig::resolved_token()));

        if decision_config.timeout_ms == 0
            || decision_config.timeout_ms > MAX_EXTERNAL_DECISION_TIMEOUT_MS
        {
            errors.push(format!(
                "ExternalDecision plugin timeout_ms must be between 1 and {}",
                MAX_EXTERNAL_DECISION_TIMEOUT_MS
            ));
        }

        (errors, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::{super::TransactionPluginRunner, *};
    use crate::{
        config::TransactionPluginType,
        constant::DEFAULT_EXTERNAL_DECISION_TIMEOUT_MS,
        tests::{common::RpcMockBuilder, config_mock::ConfigMockBuilder},
        transaction::TransactionUtil,
    };
    use mockito::{Matcher, Server};
    use serde_json::json;
    use solana_message::{Message, VersionedMessage};
    use solana_system_interface::instruction::transfer;
    use std::time::Instant;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    const ALL_CONTEXTS: [PluginExecutionContext; 4] = [
        PluginExecutionContext::SignTransaction,
        PluginExecutionContext::SignAndSendTransaction,
        PluginExecutionContext::SignBundle,
        PluginExecutionContext::SignAndSendBundle,
    ];

    fn config_with(url: &str, timeout_ms: u64) -> Config {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.plugins.enabled = vec![TransactionPluginType::ExternalDecision];
        config.kora.plugins.external_decision =
            Some(ExternalDecisionConfig { url: url.to_string(), timeout_ms });
        config
    }

    fn transaction(fee_payer: &Pubkey) -> VersionedTransactionResolved {
        let ix = transfer(fee_payer, &Pubkey::new_from_array([7; 32]), 5_000);
        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[ix], Some(fee_payer)),
        ));
        VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap()
    }

    async fn run(config: &Config, fee_payer: &Pubkey) -> Result<(), KoraError> {
        run_in(config, fee_payer, PluginExecutionContext::SignTransaction).await
    }

    async fn run_in(
        config: &Config,
        fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let rpc_client = RpcMockBuilder::new().build();
        TransactionPluginRunner::from_config(config)
            .run(&transaction(fee_payer), config, rpc_client.as_ref(), fee_payer, context)
            .await
    }

    async fn answering(status: usize, body: &str) -> (mockito::ServerGuard, mockito::Mock) {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/decide")
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;
        (server, mock)
    }

    #[tokio::test]
    async fn external_decision_allows_on_explicit_allow_and_sends_the_transaction() {
        let mut server = Server::new_async().await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        for context in ALL_CONTEXTS {
            let fee_payer = Pubkey::new_unique();
            let expected_tx = transaction(&fee_payer).encode_b64_transaction().unwrap();
            let mock = server
                .mock("POST", "/decide")
                .match_header("content-type", "application/json")
                .match_body(Matcher::Json(json!({
                    "transaction": expected_tx,
                    "fee_payer": fee_payer.to_string(),
                    "method": context.method_name(),
                })))
                .with_status(200)
                .with_body(r#"{"decision":"allow","attestation":"opaque provider data"}"#)
                .expect(1)
                .create_async()
                .await;

            let result = run_in(&config, &fee_payer, context).await;

            assert!(result.is_ok(), "{context:?}: {result:?}");
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn external_decision_deny_refuses_with_a_cleaned_reason() {
        let long_tail = "x".repeat(500);
        let (server, _mock) = answering(
            200,
            &json!({"decision": "deny", "reason": format!("drains fee payer\r\n\u{1b}[2J{long_tail}")})
                .to_string(),
        )
        .await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        let KoraError::InvalidTransaction(message) = err else {
            panic!("expected InvalidTransaction, got {err:?}");
        };
        assert!(message.contains("denied by the decision endpoint: drains fee payer  [2Jxxx"));
        assert!(!message.chars().any(|c| c.is_control()));
        let reason = message
            .split_once("denied by the decision endpoint: ")
            .and_then(|(_, rest)| rest.rsplit_once(" (context:"))
            .map(|(reason, _)| reason)
            .unwrap();
        assert_eq!(reason.chars().count(), MAX_DENY_REASON_CHARS);
    }

    #[tokio::test]
    async fn external_decision_deny_reason_drops_hidden_characters() {
        let (server, _mock) = answering(
            200,
            &json!({"decision": "deny", "reason": "fee\u{202E}payer\u{200B} drain\u{E0041}\u{2066}ed\u{2028}by\u{00AD} \u{FE0F}a\u{E0100}\u{3164}\u{FFF9}n\u{1D173} endpoint"})
                .to_string(),
        )
        .await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        let KoraError::InvalidTransaction(message) = err else {
            panic!("expected InvalidTransaction, got {err:?}");
        };
        assert!(
            message.contains("denied by the decision endpoint: feepayer drained by an endpoint"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn external_decision_deny_reason_is_capped_after_redaction() {
        // A key cut by the cap before redaction kept its first 31 hex characters, too few to redact.
        let key = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
        let (server, _mock) = answering(
            200,
            &json!({"decision": "deny", "reason": format!("{}{key}", "z".repeat(169))}).to_string(),
        )
        .await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        let KoraError::InvalidTransaction(message) = err else {
            panic!("expected InvalidTransaction, got {err:?}");
        };
        assert!(!message.contains(&key[..8]), "{message}");

        // Redaction can lengthen a reason: the cap holds for what the client receives.
        let (server, _mock) = answering(
            200,
            &json!({"decision": "deny", "reason": "http://a/ ".repeat(20)}).to_string(),
        )
        .await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);
        let KoraError::InvalidTransaction(message) =
            run(&config, &Pubkey::new_unique()).await.unwrap_err()
        else {
            panic!("expected InvalidTransaction");
        };
        let reason = message
            .split_once("denied by the decision endpoint: ")
            .and_then(|(_, rest)| rest.rsplit_once(" (context:"))
            .map(|(reason, _)| reason)
            .unwrap();
        assert!(reason.chars().count() <= MAX_DENY_REASON_CHARS, "{reason}");
    }

    #[test]
    fn external_decision_deny_reason_keeps_a_key_wrapped_over_lines_whole() {
        let key = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
        // Before this fix the break became a space before redaction, and 31 hex characters showed.
        for wrapped in [
            format!("leaked key {}\n{}", &key[..31], &key[31..]),
            format!("leaked key {}\r\n{}", &key[..60], &key[60..]),
            format!("leaked key {}\t{}\u{2028}{}", &key[..20], &key[20..40], &key[40..]),
        ] {
            let reason = deny_reason(&wrapped);
            assert!(!reason.contains(&key[60..]) && !reason.contains(&key[..8]), "{reason}");
            assert!(reason.starts_with("leaked key "), "{reason}");
        }
        // Breaks anywhere else still become spaces.
        assert_eq!(
            deny_reason("on the list\nsince today\r\n2 hits"),
            "on the list since today  2 hits"
        );
    }

    #[tokio::test]
    async fn external_decision_failure_detail_does_not_quote_the_body() {
        // serde's error quotes an unknown decision: a line break in it would forge a log line.
        let (server, _mock) =
            answering(200, &json!({"decision": "x\nINFO forged log line"}).to_string()).await;
        let config =
            ExternalDecisionConfig { url: format!("{}/decide", server.url()), timeout_ms: 2_000 };
        let body = DecisionRequest {
            transaction: String::new(),
            fee_payer: Pubkey::new_unique().to_string(),
            method: "signTransaction",
        };

        let Err(detail) = ExternalDecisionPlugin::request_decision(&config, &body, None).await
        else {
            panic!("expected an unrecognized response");
        };

        assert!(detail.starts_with("unrecognized response: "), "{detail}");
        assert!(!detail.contains('\n') && !detail.contains("forged"), "{detail}");
    }

    #[tokio::test]
    async fn external_decision_refuses_a_body_past_the_size_cap() {
        let padding = "x".repeat(MAX_RESPONSE_BYTES);
        let (server, _mock) =
            answering(200, &json!({"decision": "allow", "padding": padding}).to_string()).await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::InternalServerError(_)), "{err:?}");
    }

    #[tokio::test]
    async fn external_decision_deny_without_reason_refuses() {
        let (server, _mock) = answering(200, r#"{"decision":"deny"}"#).await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::InvalidTransaction(ref m) if m.contains("denied")));
    }

    #[tokio::test]
    async fn external_decision_refuses_an_allow_body_under_any_status_but_200() {
        for status in [201, 204, 302, 403, 500, 503] {
            let (server, mock) = answering(status, r#"{"decision":"allow"}"#).await;
            let config = config_with(&format!("{}/decide", server.url()), 2_000);

            let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

            assert!(matches!(err, KoraError::InternalServerError(_)), "{status}: {err:?}");
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn external_decision_refuses_any_body_but_a_known_decision() {
        for body in [
            "",
            "allow",
            "{}",
            "null",
            r#"{"decision":"ALLOW"}"#,
            r#"{"decision":"review"}"#,
            r#"{"decision":true}"#,
            r#"{"allow":true}"#,
            r#"{"decision":"deny","decision":"allow"}"#,
            r#"[{"decision":"allow"}]"#,
            r#"["allow"]"#,
            r#" ["allow"]"#,
            r#"["deny","reason"]"#,
        ] {
            let (server, _mock) = answering(200, body).await;
            let config = config_with(&format!("{}/decide", server.url()), 2_000);

            let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

            assert!(matches!(err, KoraError::InternalServerError(_)), "{body}: {err:?}");
        }
    }

    #[tokio::test]
    async fn external_decision_does_not_follow_redirects() {
        let mut server = Server::new_async().await;
        let redirect = server
            .mock("POST", "/decide")
            .with_status(307)
            .with_header("location", &format!("{}/elsewhere", server.url()))
            .create_async()
            .await;
        let elsewhere = server
            .mock("POST", "/elsewhere")
            .with_status(200)
            .with_body(r#"{"decision":"allow"}"#)
            .expect(0)
            .create_async()
            .await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::InternalServerError(_)));
        redirect.assert_async().await;
        elsewhere.assert_async().await;
    }

    #[tokio::test]
    async fn external_decision_times_out_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let hold = tokio::spawn(async move {
            let mut open = Vec::new();
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                open.push(socket);
            }
        });
        let config = config_with(&format!("http://{address}/decide"), 200);

        let started = Instant::now();
        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::InternalServerError(_)));
        assert!(started.elapsed() < Duration::from_secs(5));
        hold.abort();
    }

    #[tokio::test]
    async fn external_decision_refuses_when_the_endpoint_is_unreachable() {
        let address = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
        let config = config_with(&format!("http://{address}/decide"), 2_000);

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::InternalServerError(_)));
    }

    #[tokio::test]
    async fn external_decision_refuses_without_its_config_section() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.plugins.enabled = vec![TransactionPluginType::ExternalDecision];

        let err = run(&config, &Pubkey::new_unique()).await.unwrap_err();

        assert!(matches!(err, KoraError::ConfigError(_)));
    }

    #[tokio::test]
    async fn external_decision_sends_the_token_only_when_set() {
        let mut server = Server::new_async().await;
        let config =
            ExternalDecisionConfig { url: format!("{}/decide", server.url()), timeout_ms: 2_000 };
        let body = DecisionRequest {
            transaction: String::new(),
            fee_payer: Pubkey::new_unique().to_string(),
            method: "signTransaction",
        };

        let with_token = server
            .mock("POST", "/decide")
            .match_header("authorization", "Bearer test-decision-token")
            .with_status(200)
            .with_body(r#"{"decision":"allow"}"#)
            .expect(1)
            .create_async()
            .await;
        let result =
            ExternalDecisionPlugin::request_decision(&config, &body, Some("test-decision-token"))
                .await;
        assert!(matches!(result, Ok(Decision::Allow)));
        with_token.assert_async().await;
        with_token.remove_async().await;

        let without_token = server
            .mock("POST", "/decide")
            .match_header("authorization", Matcher::Missing)
            .with_status(200)
            .with_body(r#"{"decision":"allow"}"#)
            .expect(1)
            .create_async()
            .await;
        let result = ExternalDecisionPlugin::request_decision(&config, &body, None).await;
        assert!(matches!(result, Ok(Decision::Allow)));
        without_token.assert_async().await;
    }

    #[tokio::test]
    async fn external_decision_failure_detail_does_not_carry_the_url() {
        let address = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
        let config = ExternalDecisionConfig {
            url: format!("http://{address}/hooks/kora.decide/secret-path-token;key=secret"),
            timeout_ms: 2_000,
        };
        let body = DecisionRequest {
            transaction: String::new(),
            fee_payer: Pubkey::new_unique().to_string(),
            method: "signTransaction",
        };

        let Err(detail) = ExternalDecisionPlugin::request_decision(&config, &body, None).await
        else {
            panic!("expected a transport error");
        };

        assert!(detail.starts_with("request failed"), "{detail}");
        assert!(!detail.contains("secret"), "{detail}");
    }

    #[tokio::test]
    async fn external_decision_reaches_a_loopback_endpoint_without_a_proxy() {
        for url in
            ["http://127.0.0.1:9000/decide", "http://localhost/decide", "http://[::1]/decide"]
        {
            let url = Url::parse(url).unwrap();
            assert!(std::ptr::eq(client_for(&url), &*LOOPBACK_CLIENT), "{url}");
        }
        let remote = Url::parse("https://decisions.example.com/decide").unwrap();
        assert!(std::ptr::eq(client_for(&remote), &*HTTP_CLIENT));

        // A proxy that answers allow to anything, and an endpoint that denies. The proxy is set on
        // the builder rather than in the environment, which other tests share.
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = proxy.accept().await {
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request).await;
                let body = r#"{"decision":"allow"}"#;
                let answer = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(answer.as_bytes()).await;
            }
        });
        let (server, _mock) = answering(200, r#"{"decision":"deny"}"#).await;
        let answer = |loopback: bool| {
            let client = build_client(
                Client::builder().proxy(reqwest::Proxy::all(&proxy_url).unwrap()),
                loopback,
            );
            let url = format!("{}/decide", server.url());
            async move { client.post(url).send().await.unwrap().text().await.unwrap() }
        };

        assert_eq!(answer(true).await, r#"{"decision":"deny"}"#, "the loopback client goes direct");
        assert_eq!(answer(false).await, r#"{"decision":"allow"}"#, "the proxy would have answered");
    }

    #[tokio::test]
    async fn external_decision_token_that_cannot_be_read_asks_nothing() {
        let mut server = Server::new_async().await;
        let config =
            ExternalDecisionConfig { url: format!("{}/decide", server.url()), timeout_ms: 2_000 };
        let body = DecisionRequest {
            transaction: String::new(),
            fee_payer: Pubkey::new_unique().to_string(),
            method: "signTransaction",
        };
        let never = server
            .mock("POST", "/decide")
            .with_status(200)
            .with_body(r#"{"decision":"allow"}"#)
            .expect(0)
            .create_async()
            .await;

        let error = "KORA_EXTERNAL_DECISION_TOKEN is not valid UTF-8".to_string();
        let result = ExternalDecisionPlugin::decide(&config, &body, Err(error.clone())).await;
        assert!(matches!(result, Err(ref detail) if *detail == error));
        never.assert_async().await;
        // And validation reports it, by the variable's name only.
        assert_eq!(
            token_problem(&Err(error.clone())),
            Some(format!("ExternalDecision plugin: {error}"))
        );
        assert_eq!(token_problem(&Ok(None)), None);
        assert!(token_problem(&Ok(Some("abc\n".to_string()))).is_some());
    }

    #[test]
    fn external_decision_loopback_client_has_no_proxy() {
        // reqwest lists a client's proxies in its Debug output, the environment's included, and
        // `no_proxy` empties that list: the static the plugin uses for loopback must have none.
        assert!(!format!("{:?}", *LOOPBACK_CLIENT).contains("proxies"));
        assert!(format!("{:?}", *HTTP_CLIENT).contains("proxies"));
    }

    #[cfg(unix)]
    #[test]
    fn external_decision_token_that_is_not_utf8_is_an_error() {
        use std::{env::VarError, ffi::OsString, os::unix::ffi::OsStringExt};

        assert_eq!(ExternalDecisionConfig::token_from(Err(VarError::NotPresent)), Ok(None));
        assert_eq!(ExternalDecisionConfig::token_from(Ok(String::new())), Ok(None));
        assert_eq!(
            ExternalDecisionConfig::token_from(Ok("abc123".to_string())),
            Ok(Some("abc123".to_string()))
        );
        // Before this fix such a token was read as no token, and requests went out without it.
        let not_utf8 = VarError::NotUnicode(OsString::from_vec(b"secret\xff".to_vec()));
        let error = ExternalDecisionConfig::token_from(Err(not_utf8)).unwrap_err();
        assert!(error.contains(ExternalDecisionConfig::TOKEN_ENV));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn external_decision_config_rejects_a_token_that_is_not_a_header_value() {
        assert_eq!(token_error(None), None);
        assert_eq!(token_error(Some("abc123")), None);
        for token in ["abc123\n", "abc\r\n123"] {
            let error = token_error(Some(token)).unwrap();
            assert!(error.contains(ExternalDecisionConfig::TOKEN_ENV));
            assert!(!error.contains("abc"));
        }
    }

    fn config_errors(config: &Config) -> Vec<String> {
        TransactionPluginRunner::validate_config(config).0
    }

    #[test]
    fn external_decision_config_accepts_https_and_loopback_http() {
        for url in [
            "https://decisions.example.com/v1/decide",
            "http://127.0.0.1:9000/decide",
            "http://localhost:9000/decide",
            "http://[::1]:9000/decide",
        ] {
            let config = config_with(url, DEFAULT_EXTERNAL_DECISION_TIMEOUT_MS);
            assert!(config_errors(&config).is_empty(), "{url}: {:?}", config_errors(&config));
        }
    }

    #[test]
    fn external_decision_config_rejects_bad_urls_without_echoing_them() {
        for (url, expected) in [
            ("http://decisions.example.com/decide?key=secret", "must use https"),
            ("ftp://decisions.example.com/secret", "must use https"),
            ("https://user:secret@decisions.example.com/decide", "must not embed credentials"),
            ("not a url secret", "not a valid URL"),
            ("", "not a valid URL"),
        ] {
            let errors = config_errors(&config_with(url, DEFAULT_EXTERNAL_DECISION_TIMEOUT_MS));
            assert!(errors.iter().any(|e| e.contains(expected)), "{url}: {errors:?}");
            assert!(errors.iter().all(|e| !e.contains("secret")), "{url}: {errors:?}");
        }
    }

    #[test]
    fn external_decision_config_bounds_the_timeout() {
        let url = "https://decisions.example.com/decide";
        assert!(config_errors(&config_with(url, 1)).is_empty());
        assert!(config_errors(&config_with(url, MAX_EXTERNAL_DECISION_TIMEOUT_MS)).is_empty());
        for timeout_ms in [0, MAX_EXTERNAL_DECISION_TIMEOUT_MS + 1] {
            let errors = config_errors(&config_with(url, timeout_ms));
            assert!(errors.iter().any(|e| e.contains("timeout_ms")), "{timeout_ms}: {errors:?}");
        }
    }

    #[test]
    fn external_decision_config_requires_its_section() {
        let mut config = ConfigMockBuilder::new().build();
        config.kora.plugins.enabled = vec![TransactionPluginType::ExternalDecision];

        let errors = config_errors(&config);

        assert!(errors.iter().any(|e| e.contains("requires [kora.plugins.external_decision]")));
    }
}
