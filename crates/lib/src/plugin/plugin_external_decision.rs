use std::{sync::LazyLock, time::Duration};

use async_trait::async_trait;
use reqwest::{header::HeaderValue, redirect::Policy, Client, StatusCode};
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

// Redirects are not followed, so the transaction and the token only reach the configured URL.
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder().redirect(Policy::none()).build().expect("Failed to build reqwest client")
});

// A loopback endpoint is reached directly. A proxy from the environment (HTTP_PROXY, ALL_PROXY)
// would otherwise receive the plain-HTTP request, token included, and answer in its place.
static LOOPBACK_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .build()
        .expect("Failed to build reqwest client")
});

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

fn deny_reason(reason: &str) -> String {
    let reason: String =
        reason.chars().filter(|c| !c.is_control()).take(MAX_DENY_REASON_CHARS).collect();
    sanitize_error!(reason)
}

/// Asks an operator-run HTTP endpoint whether the fee payer may sign a transaction. Like the other
/// plugins, it runs after `validate_transaction` and before the payment, fee and transfer-hook
/// checks, so the endpoint can be asked about a transaction Kora then refuses. The endpoint
/// receives the transaction as the client submitted it, the fee payer, and the RPC method. Only
/// an HTTP 200 answer of `{"decision": "allow"}` lets the transaction through; a deny, a timeout,
/// a transport error, any other status, or any other body refuses it.
///
/// Use case: a risk or policy service that catches fee payer abuse the structural checks miss,
/// without tying Kora to a particular provider.
pub(super) struct ExternalDecisionPlugin;

impl ExternalDecisionPlugin {
    async fn request_decision(
        config: &ExternalDecisionConfig,
        body: &DecisionRequest,
    ) -> Result<Decision, String> {
        let url = Url::parse(&config.url).map_err(|_| "the url is not a valid URL".to_string())?;
        let mut request =
            client_for(&url).post(url).timeout(Duration::from_millis(config.timeout_ms)).json(body);
        if let Some(token) = ExternalDecisionConfig::resolved_token() {
            request = request.bearer_auth(token);
        }

        // Errors are logged without their url, which may carry a credential.
        let response =
            request.send().await.map_err(|e| format!("request failed: {}", e.without_url()))?;
        let status = response.status();
        if status != StatusCode::OK {
            return Err(format!("endpoint returned HTTP {}", status.as_u16()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("failed to read response body: {}", e.without_url()))?;
        // An object only: serde also reads a tagged enum from an array such as ["allow"].
        if bytes.trim_ascii_start().first() != Some(&b'{') {
            return Err("unrecognized response: not a JSON object".to_string());
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("unrecognized response: {e}"))
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

        match Self::request_decision(decision_config, &body).await {
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

        if let Some(error) = token_error(ExternalDecisionConfig::resolved_token().as_deref()) {
            errors.push(error);
        }

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
    use serial_test::serial;
    use solana_message::{Message, VersionedMessage};
    use solana_system_interface::instruction::transfer;
    use std::time::Instant;
    use tokio::net::TcpListener;

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
        assert!(message.contains("denied by the decision endpoint: drains fee payer[2Jxxx"));
        assert!(!message.chars().any(|c| c.is_control()));
        assert!(!message.contains(&"x".repeat(MAX_DENY_REASON_CHARS)));
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
    #[serial]
    async fn external_decision_sends_the_token_from_the_environment_only_when_set() {
        let mut server = Server::new_async().await;
        let config = config_with(&format!("{}/decide", server.url()), 2_000);

        std::env::set_var(ExternalDecisionConfig::TOKEN_ENV, "test-decision-token");
        let with_token = server
            .mock("POST", "/decide")
            .match_header("authorization", "Bearer test-decision-token")
            .with_status(200)
            .with_body(r#"{"decision":"allow"}"#)
            .expect(1)
            .create_async()
            .await;
        let result = run(&config, &Pubkey::new_unique()).await;
        std::env::remove_var(ExternalDecisionConfig::TOKEN_ENV);
        assert!(result.is_ok(), "{result:?}");
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
        let result = run(&config, &Pubkey::new_unique()).await;
        assert!(result.is_ok(), "{result:?}");
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

        let Err(detail) = ExternalDecisionPlugin::request_decision(&config, &body).await else {
            panic!("expected a transport error");
        };

        assert!(detail.starts_with("request failed"), "{detail}");
        assert!(!detail.contains("secret"), "{detail}");
    }

    #[test]
    fn external_decision_reaches_a_loopback_endpoint_without_an_environment_proxy() {
        for url in
            ["http://127.0.0.1:9000/decide", "http://localhost/decide", "http://[::1]/decide"]
        {
            let url = Url::parse(url).unwrap();
            assert!(std::ptr::eq(client_for(&url), &*LOOPBACK_CLIENT), "{url}");
        }
        let remote = Url::parse("https://decisions.example.com/decide").unwrap();
        assert!(std::ptr::eq(client_for(&remote), &*HTTP_CLIENT));
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
