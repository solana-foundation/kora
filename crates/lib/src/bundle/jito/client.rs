use crate::{
    bundle::{
        jito::{config::JitoConfig, constant::JITO_MOCK_BLOCK_ENGINE_URL, error::JitoError},
        BundleError,
    },
    sanitize_error,
};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

pub enum JitoBundleClient {
    Live(JitoClient),
    Mock(JitoMockClient),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitoBundleAccountConfig {
    pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct JitoBundleSimulationConfig {
    #[serde(
        rename = "preExecutionAccountsConfigs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pre_execution_accounts_configs: Option<Vec<Option<JitoBundleAccountConfig>>>,
    #[serde(
        rename = "postExecutionAccountsConfigs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub post_execution_accounts_configs: Option<Vec<Option<JitoBundleAccountConfig>>>,
    #[serde(rename = "transactionEncoding", default, skip_serializing_if = "Option::is_none")]
    pub transaction_encoding: Option<String>,
    #[serde(rename = "skipSigVerify", default, skip_serializing_if = "Option::is_none")]
    pub skip_sig_verify: Option<bool>,
    #[serde(rename = "replaceRecentBlockhash", default, skip_serializing_if = "Option::is_none")]
    pub replace_recent_blockhash: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitoBundleSimulationTransactionResult {
    pub err: Option<Value>,
    #[serde(default)]
    pub logs: Vec<String>,
    pub context: Value,
    pub pre_execution_accounts: Option<Vec<Value>>,
    pub post_execution_accounts: Option<Vec<Value>>,
    pub units_consumed: Option<u64>,
    pub return_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitoBundleSimulationResult {
    pub context: Value,
    pub summary: Option<Value>,
    pub transaction_results: Vec<JitoBundleSimulationTransactionResult>,
}

impl JitoBundleSimulationResult {
    fn parse_transaction_result(
        value: Value,
    ) -> Result<JitoBundleSimulationTransactionResult, JitoError> {
        let logs = match value.get("logs") {
            Some(Value::Array(entries)) => entries
                .iter()
                .map(|entry| {
                    entry.as_str().map(ToString::to_string).ok_or_else(|| {
                        JitoError::ApiError(
                            "Invalid log entry in simulateBundle response".to_string(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            None | Some(Value::Null) => {
                return Err(JitoError::ApiError(
                    "Missing logs in simulateBundle response".to_string(),
                ))
            }
            _ => {
                return Err(JitoError::ApiError(
                    "Invalid logs in simulateBundle response".to_string(),
                ))
            }
        };

        let pre_execution_accounts = match value.get("preExecutionAccounts") {
            None | Some(Value::Null) => None,
            Some(Value::Array(accounts)) => Some(accounts.clone()),
            _ => {
                return Err(JitoError::ApiError(
                    "Invalid preExecutionAccounts in simulateBundle response".to_string(),
                ))
            }
        };

        let post_execution_accounts = match value.get("postExecutionAccounts") {
            None | Some(Value::Null) => None,
            Some(Value::Array(accounts)) => Some(accounts.clone()),
            _ => {
                return Err(JitoError::ApiError(
                    "Invalid postExecutionAccounts in simulateBundle response".to_string(),
                ))
            }
        };

        Ok(JitoBundleSimulationTransactionResult {
            err: value.get("err").filter(|err| !err.is_null()).cloned(),
            logs,
            context: Value::Null,
            pre_execution_accounts,
            post_execution_accounts,
            units_consumed: value.get("unitsConsumed").and_then(Value::as_u64),
            return_data: value.get("returnData").filter(|data| !data.is_null()).cloned(),
        })
    }

    fn from_rpc_result(result: Value) -> Result<Self, JitoError> {
        let context = result.get("context").cloned().unwrap_or(Value::Null);

        if let Some(value) = result.get("value").cloned() {
            let summary = value.get("summary").cloned();

            if let Some(transaction_results) =
                value.get("transactionResults").and_then(Value::as_array)
            {
                let mut parsed_results = Vec::with_capacity(transaction_results.len());
                for tx_result in transaction_results {
                    let mut parsed = Self::parse_transaction_result(tx_result.clone())?;
                    parsed.context = context.clone();
                    parsed_results.push(parsed);
                }
                return Ok(Self { context, summary, transaction_results: parsed_results });
            }

            let mut parsed = Self::parse_transaction_result(value)?;
            parsed.context = context.clone();
            return Ok(Self { context, summary: None, transaction_results: vec![parsed] });
        }

        if result.get("err").is_some()
            || result.get("logs").is_some()
            || result.get("preExecutionAccounts").is_some()
            || result.get("postExecutionAccounts").is_some()
        {
            let mut parsed = Self::parse_transaction_result(result)?;
            parsed.context = context.clone();
            return Ok(Self { context, summary: None, transaction_results: vec![parsed] });
        }

        Err(JitoError::ApiError(
            "Invalid simulateBundle response: missing `value` or transaction fields".to_string(),
        ))
    }

    fn summary_failed(summary: &Value) -> bool {
        match summary {
            Value::String(status) => !status.eq_ignore_ascii_case("succeeded"),
            Value::Object(map) => map.contains_key("failed") || map.contains_key("error"),
            _ => false,
        }
    }

    fn first_failure(&self) -> Option<(usize, &JitoBundleSimulationTransactionResult)> {
        self.transaction_results.iter().enumerate().find(|(_, tx_result)| tx_result.err.is_some())
    }
}

impl JitoBundleClient {
    pub fn new(config: &JitoConfig) -> Self {
        if config.block_engine_url == JITO_MOCK_BLOCK_ENGINE_URL {
            Self::Mock(JitoMockClient::new())
        } else {
            Self::Live(JitoClient::new(config))
        }
    }

    pub async fn send_bundle(
        &self,
        encoded_transactions: &[String],
    ) -> Result<String, BundleError> {
        match self {
            Self::Live(client) => client.send_bundle(encoded_transactions).await,
            Self::Mock(client) => client.send_bundle(encoded_transactions).await,
        }
    }

    pub async fn get_bundle_statuses(
        &self,
        bundle_uuids: Vec<String>,
    ) -> Result<Value, BundleError> {
        match self {
            Self::Live(client) => client.get_bundle_statuses(bundle_uuids).await,
            Self::Mock(client) => client.get_bundle_statuses(bundle_uuids).await,
        }
    }

    pub async fn simulate_bundle(
        &self,
        encoded_transactions: &[String],
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        match self {
            Self::Live(client) => client.simulate_bundle(encoded_transactions).await,
            Self::Mock(client) => client.simulate_bundle(encoded_transactions).await,
        }
    }

    pub async fn simulate_bundle_with_config(
        &self,
        encoded_transactions: &[String],
        simulation_config: JitoBundleSimulationConfig,
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        match self {
            Self::Live(client) => {
                client.simulate_bundle_with_config(encoded_transactions, simulation_config).await
            }
            Self::Mock(client) => {
                client.simulate_bundle_with_config(encoded_transactions, simulation_config).await
            }
        }
    }
}

pub struct JitoClient {
    block_engine_url: String,
    simulate_bundle_url: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

impl JitoClient {
    pub fn new(config: &JitoConfig) -> Self {
        Self {
            block_engine_url: config.block_engine_url.clone(),
            simulate_bundle_url: config
                .simulate_bundle_url
                .clone()
                .unwrap_or_else(|| config.block_engine_url.clone()),
            client: Client::new(),
        }
    }

    fn block_engine_base_url(&self) -> &str {
        self.block_engine_url.trim_end_matches('/')
    }

    fn bundle_api_url(&self) -> String {
        format!("{}/api/v1/bundles", self.block_engine_base_url())
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        url: &str,
        method: &str,
        params: Value,
    ) -> Result<T, JitoError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                JitoError::ApiError(format!("HTTP request failed: {}", sanitize_error!(e)))
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(JitoError::ApiError(format!("HTTP error: {}", status)));
        }

        let rpc_response: JsonRpcResponse = response.json().await.map_err(|e| {
            JitoError::ApiError(format!("Failed to parse response: {}", sanitize_error!(e)))
        })?;

        if let Some(error) = rpc_response.error {
            return Err(JitoError::ApiError(error.message));
        }

        let result = rpc_response
            .result
            .ok_or_else(|| JitoError::ApiError("Empty response from Jito".to_string()))?;

        serde_json::from_value(result).map_err(|e| {
            JitoError::ApiError(format!("Failed to parse {method} result: {}", sanitize_error!(e)))
        })
    }

    pub async fn send_bundle(
        &self,
        encoded_transactions: &[String],
    ) -> Result<String, BundleError> {
        let params = json!([encoded_transactions, {"encoding": "base64"}]);
        let url = self.bundle_api_url();
        Ok(self.send_request(&url, "sendBundle", params).await?)
    }

    pub async fn get_bundle_statuses(
        &self,
        bundle_uuids: Vec<String>,
    ) -> Result<Value, BundleError> {
        let params = json!([bundle_uuids]);
        let url = self.bundle_api_url();
        Ok(self.send_request(&url, "getBundleStatuses", params).await?)
    }

    pub async fn simulate_bundle(
        &self,
        encoded_transactions: &[String],
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        self.simulate_bundle_with_optional_config(encoded_transactions, None).await
    }

    pub async fn simulate_bundle_with_config(
        &self,
        encoded_transactions: &[String],
        simulation_config: JitoBundleSimulationConfig,
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        self.simulate_bundle_with_optional_config(encoded_transactions, Some(simulation_config))
            .await
    }

    async fn simulate_bundle_with_optional_config(
        &self,
        encoded_transactions: &[String],
        simulation_config: Option<JitoBundleSimulationConfig>,
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        let mut params = vec![json!({ "encodedTransactions": encoded_transactions })];
        if let Some(config) = simulation_config {
            params.push(serde_json::to_value(config).map_err(|e| {
                JitoError::ApiError(format!(
                    "Failed to serialize simulateBundle config: {}",
                    sanitize_error!(e)
                ))
            })?);
        }

        let simulate_url = self.simulate_bundle_url.trim_end_matches('/');
        let raw_result: Value =
            self.send_request(simulate_url, "simulateBundle", Value::Array(params)).await?;
        let parsed_result = JitoBundleSimulationResult::from_rpc_result(raw_result)?;

        if let Some((failed_idx, failed_result)) = parsed_result.first_failure() {
            let mut message = format!(
                "Bundle simulation failed at transaction index {}: {}",
                failed_idx,
                failed_result.err.as_ref().unwrap_or(&Value::Null)
            );
            if !failed_result.logs.is_empty() {
                message.push_str(&format!(". Logs: {}", failed_result.logs.join(" | ")));
            }
            return Err(BundleError::ValidationError(message));
        }

        if let Some(summary) = &parsed_result.summary {
            if JitoBundleSimulationResult::summary_failed(summary) {
                return Err(BundleError::ValidationError(format!(
                    "Bundle simulation failed: {summary}",
                )));
            }
        }

        Ok(parsed_result)
    }
}

pub struct JitoMockClient;

impl JitoMockClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_bundle(
        &self,
        _encoded_transactions: &[String],
    ) -> Result<String, BundleError> {
        let random_id: u64 = rand::random();
        Ok(format!("mock-bundle-{random_id}"))
    }

    pub async fn get_bundle_statuses(
        &self,
        bundle_uuids: Vec<String>,
    ) -> Result<Value, BundleError> {
        let mock_statuses: Vec<Value> = bundle_uuids
            .iter()
            .map(|uuid| {
                json!({
                    "bundle_id": uuid,
                    "status": "Landed",
                    "landed_slot": 12345678
                })
            })
            .collect();
        Ok(json!({ "value": mock_statuses }))
    }

    pub async fn simulate_bundle(
        &self,
        _encoded_transactions: &[String],
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        self.simulate_bundle_with_config(
            _encoded_transactions,
            JitoBundleSimulationConfig::default(),
        )
        .await
    }

    pub async fn simulate_bundle_with_config(
        &self,
        encoded_transactions: &[String],
        simulation_config: JitoBundleSimulationConfig,
    ) -> Result<JitoBundleSimulationResult, BundleError> {
        let transaction_results = encoded_transactions
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let pre_accounts = simulation_config
                    .pre_execution_accounts_configs
                    .as_ref()
                    .and_then(|configs| configs.get(idx))
                    .and_then(|entry| entry.as_ref())
                    .map(|entry| {
                        entry
                            .addresses
                            .iter()
                            .map(|_| {
                                json!({
                                    "lamports": 1_000_000_000_u64,
                                    "owner": "11111111111111111111111111111111",
                                    "data": ["", "base64"],
                                    "executable": false,
                                    "rentEpoch": 0_u64
                                })
                            })
                            .collect::<Vec<Value>>()
                    });

                let post_accounts = simulation_config
                    .post_execution_accounts_configs
                    .as_ref()
                    .and_then(|configs| configs.get(idx))
                    .and_then(|entry| entry.as_ref())
                    .map(|entry| {
                        entry
                            .addresses
                            .iter()
                            .map(|_| {
                                json!({
                                    "lamports": 1_000_000_000_u64,
                                    "owner": "11111111111111111111111111111111",
                                    "data": ["", "base64"],
                                    "executable": false,
                                    "rentEpoch": 0_u64
                                })
                            })
                            .collect::<Vec<Value>>()
                    });

                JitoBundleSimulationTransactionResult {
                    err: None,
                    logs: vec![],
                    context: json!({ "slot": 12345678, "apiVersion": "mock" }),
                    pre_execution_accounts: pre_accounts,
                    post_execution_accounts: post_accounts,
                    units_consumed: Some(0),
                    return_data: None,
                }
            })
            .collect::<Vec<JitoBundleSimulationTransactionResult>>();

        Ok(JitoBundleSimulationResult {
            context: json!({ "slot": 12345678, "apiVersion": "mock" }),
            summary: Some(json!("succeeded")),
            transaction_results,
        })
    }
}

impl Default for JitoMockClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::transaction_mock::create_mock_encoded_transaction;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn test_send_bundle_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({"method": "sendBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"bundle-uuid-12345"}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "bundle-uuid-12345");
    }

    #[tokio::test]
    async fn test_send_bundle_http_error() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(500)
            .with_body("Internal Server Error")
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_bundle_rpc_error() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"error":{"message":"Bundle rejected"}}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_bundle_statuses_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .match_body(Matcher::PartialJson(json!({"method": "getBundleStatuses"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"value":[{"bundle_id":"uuid1","status":"Landed"}]}}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.get_bundle_statuses(vec!["uuid1".to_string()]).await;
        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_simulate_bundle_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({
                "method": "simulateBundle",
                "params": [{ "encodedTransactions": ["encoded-tx"] }]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123,"apiVersion":"1.18.0"},"err":null,"logs":["ok"],"unitsConsumed":42}}"#,
            )
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.simulate_bundle(&["encoded-tx".to_string()]).await;
        mock.assert();
        assert!(result.is_ok());
        let simulation = result.unwrap();
        assert_eq!(simulation.transaction_results.len(), 1);
        assert_eq!(simulation.transaction_results[0].logs, vec!["ok".to_string()]);
        assert_eq!(simulation.transaction_results[0].units_consumed, Some(42));
    }

    #[tokio::test]
    async fn test_simulate_bundle_success_with_transaction_results_shape() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({
                "method": "simulateBundle",
                "params": [{ "encodedTransactions": ["encoded-tx"] }]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":456,"apiVersion":"2.3.0"},"value":{"summary":"succeeded","transactionResults":[{"err":null,"logs":["ok-v2"],"unitsConsumed":84}]}}}"#,
            )
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.simulate_bundle(&["encoded-tx".to_string()]).await;
        mock.assert();
        assert!(result.is_ok());
        let simulation = result.unwrap();
        assert_eq!(simulation.summary, Some(json!("succeeded")));
        assert_eq!(simulation.transaction_results.len(), 1);
        assert_eq!(simulation.transaction_results[0].logs, vec!["ok-v2".to_string()]);
        assert_eq!(simulation.transaction_results[0].units_consumed, Some(84));
    }

    #[tokio::test]
    async fn test_simulate_bundle_execution_failure() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/")
            .match_body(Matcher::PartialJson(json!({"method": "simulateBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"err":"custom program error","logs":["failed log"]}}"#,
            )
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.simulate_bundle(&[create_mock_encoded_transaction()]).await;
        mock.assert();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Bundle simulation failed"));
        assert!(err.contains("failed log"));
    }

    #[tokio::test]
    async fn test_simulate_bundle_summary_failure_without_transaction_results() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/")
            .match_body(Matcher::PartialJson(json!({"method": "simulateBundle"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"value":{"summary":{"failed":{"error":"custom error"}},"transactionResults":[]}}}"#,
            )
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.simulate_bundle(&[create_mock_encoded_transaction()]).await;
        mock.assert();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Bundle simulation failed"));
    }

    #[tokio::test]
    async fn test_simulate_bundle_with_config_includes_second_param() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({
                "method": "simulateBundle",
                "params": [
                    {"encodedTransactions": ["encoded-tx"]},
                    {
                        "preExecutionAccountsConfigs": [{"addresses": ["11111111111111111111111111111111"]}],
                        "postExecutionAccountsConfigs": [{"addresses": ["11111111111111111111111111111111"]}]
                    }
                ]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"result":{"context":{"slot":123},"err":null,"logs":[]}}"#,
            )
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let simulation_config = JitoBundleSimulationConfig {
            pre_execution_accounts_configs: Some(vec![Some(JitoBundleAccountConfig {
                addresses: vec!["11111111111111111111111111111111".to_string()],
                encoding: None,
            })]),
            post_execution_accounts_configs: Some(vec![Some(JitoBundleAccountConfig {
                addresses: vec!["11111111111111111111111111111111".to_string()],
                encoding: None,
            })]),
            transaction_encoding: None,
            skip_sig_verify: None,
            replace_recent_blockhash: None,
        };

        let result = client
            .simulate_bundle_with_config(&["encoded-tx".to_string()], simulation_config)
            .await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_bundle_invalid_uuid_response() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":12345}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_bundle_empty_result() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_bundle_multiple_transactions() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"multi-tx-bundle-uuid"}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let txs = vec![
            create_mock_encoded_transaction(),
            create_mock_encoded_transaction(),
            create_mock_encoded_transaction(),
        ];

        let result = client.send_bundle(&txs).await;
        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "multi-tx-bundle-uuid");
    }

    #[tokio::test]
    async fn test_send_bundle_malformed_json_response() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"not valid json"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let tx = create_mock_encoded_transaction();

        let result = client.send_bundle(&[tx]).await;
        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_bundle_statuses_empty_uuids() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .match_body(Matcher::PartialJson(
                json!({"method": "getBundleStatuses", "params": [[]]}),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"value":[]}}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client.get_bundle_statuses(vec![]).await;
        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_bundle_statuses_multiple_uuids() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/bundles")
            .match_body(Matcher::PartialJson(json!({
                "method": "getBundleStatuses",
                "params": [["uuid1", "uuid2", "uuid3"]]
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"value":[{"bundle_id":"uuid1","status":"Landed"},{"bundle_id":"uuid2","status":"Pending"}]}}"#)
            .create();

        let config =
            JitoConfig { block_engine_url: server.url(), simulate_bundle_url: Some(server.url()) };
        let client = JitoClient::new(&config);

        let result = client
            .get_bundle_statuses(vec![
                "uuid1".to_string(),
                "uuid2".to_string(),
                "uuid3".to_string(),
            ])
            .await;
        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jito_bundle_client_dispatches_to_mock() {
        let config = JitoConfig {
            block_engine_url: JITO_MOCK_BLOCK_ENGINE_URL.to_string(),
            simulate_bundle_url: Some(JITO_MOCK_BLOCK_ENGINE_URL.to_string()),
        };
        let client = JitoBundleClient::new(&config);

        assert!(matches!(client, JitoBundleClient::Mock(_)));

        let tx = create_mock_encoded_transaction();
        let result = client.send_bundle(&[tx]).await;

        assert!(result.is_ok());
        let uuid = result.unwrap();
        assert!(uuid.starts_with("mock-bundle-"), "Expected mock UUID prefix");
    }

    #[tokio::test]
    async fn test_jito_bundle_client_dispatches_to_real() {
        let config = JitoConfig {
            block_engine_url: "https://example.com".to_string(),
            simulate_bundle_url: None,
        };
        let client = JitoBundleClient::new(&config);

        assert!(matches!(client, JitoBundleClient::Live(_)));
    }

    #[tokio::test]
    async fn test_mock_client_get_bundle_statuses() {
        let config = JitoConfig {
            block_engine_url: JITO_MOCK_BLOCK_ENGINE_URL.to_string(),
            simulate_bundle_url: Some(JITO_MOCK_BLOCK_ENGINE_URL.to_string()),
        };
        let client = JitoBundleClient::new(&config);

        let result = client.get_bundle_statuses(vec!["test-uuid".to_string()]).await;

        assert!(result.is_ok());
        let statuses = result.unwrap();
        assert!(statuses["value"].is_array());
        assert_eq!(statuses["value"][0]["status"], "Landed");
    }

    #[tokio::test]
    async fn test_mock_client_simulate_bundle() {
        let config = JitoConfig {
            block_engine_url: JITO_MOCK_BLOCK_ENGINE_URL.to_string(),
            simulate_bundle_url: Some(JITO_MOCK_BLOCK_ENGINE_URL.to_string()),
        };
        let client = JitoBundleClient::new(&config);

        let result = client.simulate_bundle(&[create_mock_encoded_transaction()]).await;

        assert!(result.is_ok());
        let simulation = result.unwrap();
        assert_eq!(simulation.summary, Some(json!("succeeded")));
        assert_eq!(simulation.transaction_results.len(), 1);
        assert!(simulation.transaction_results[0].err.is_none());
        assert_eq!(simulation.context["apiVersion"], "mock");
    }
}
