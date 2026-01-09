use crate::{
    bundle::{
        jito::{config::JitoConfig, constant::JITO_MOCK_BLOCK_ENGINE_URL, error::JitoError},
        BundleError,
    },
    sanitize_error,
    transaction::VersionedTransactionResolved,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use bincode::serialize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub enum JitoBundleClient {
    Live(JitoClient),
    Mock(JitoMockClient),
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
        transactions: &[VersionedTransactionResolved],
    ) -> Result<String, BundleError> {
        match self {
            Self::Live(client) => client.send_bundle(transactions).await,
            Self::Mock(client) => client.send_bundle(transactions).await,
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
}

pub struct JitoClient {
    block_engine_url: String,
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
        Self { block_engine_url: config.block_engine_url.clone(), client: Client::new() }
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value, JitoError> {
        let url = format!("{}/api/v1/bundles", self.block_engine_url);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let response = self
            .client
            .post(&url)
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

        rpc_response
            .result
            .ok_or_else(|| JitoError::ApiError("Empty response from Jito".to_string()))
    }

    pub async fn send_bundle(
        &self,
        transactions: &[VersionedTransactionResolved],
    ) -> Result<String, BundleError> {
        let mut encoded_txs = Vec::with_capacity(transactions.len());
        for resolved in transactions {
            let serialized = serialize(&resolved.transaction)
                .map_err(|e| BundleError::SerializationError(e.to_string()))?;
            encoded_txs.push(BASE64_STANDARD.encode(&serialized));
        }

        let params = json!([encoded_txs, {"encoding": "base64"}]);
        let result = self.send_request("sendBundle", params).await?;

        result.as_str().map(|s| s.to_string()).ok_or_else(|| {
            JitoError::ApiError("Invalid bundle UUID in response".to_string()).into()
        })
    }

    pub async fn get_bundle_statuses(
        &self,
        bundle_uuids: Vec<String>,
    ) -> Result<Value, BundleError> {
        let params = json!([bundle_uuids]);
        Ok(self.send_request("getBundleStatuses", params).await?)
    }
}

pub struct JitoMockClient;

impl JitoMockClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_bundle(
        &self,
        _transactions: &[VersionedTransactionResolved],
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
}

impl Default for JitoMockClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::transaction_mock::create_mock_resolved_transaction;
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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let result = client.get_bundle_statuses(vec!["uuid1".to_string()]).await;
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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let txs = vec![
            create_mock_resolved_transaction(),
            create_mock_resolved_transaction(),
            create_mock_resolved_transaction(),
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

        let config = JitoConfig { block_engine_url: server.url() };
        let client = JitoClient::new(&config);

        let tx = create_mock_resolved_transaction();

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

        let config = JitoConfig { block_engine_url: server.url() };
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

        let config = JitoConfig { block_engine_url: server.url() };
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
        let config = JitoConfig { block_engine_url: JITO_MOCK_BLOCK_ENGINE_URL.to_string() };
        let client = JitoBundleClient::new(&config);

        assert!(matches!(client, JitoBundleClient::Mock(_)));

        let tx = create_mock_resolved_transaction();
        let result = client.send_bundle(&[tx]).await;

        assert!(result.is_ok());
        let uuid = result.unwrap();
        assert!(uuid.starts_with("mock-bundle-"), "Expected mock UUID prefix");
    }

    #[tokio::test]
    async fn test_jito_bundle_client_dispatches_to_real() {
        let config = JitoConfig { block_engine_url: "https://example.com".to_string() };
        let client = JitoBundleClient::new(&config);

        assert!(matches!(client, JitoBundleClient::Live(_)));
    }

    #[tokio::test]
    async fn test_mock_client_get_bundle_statuses() {
        let config = JitoConfig { block_engine_url: JITO_MOCK_BLOCK_ENGINE_URL.to_string() };
        let client = JitoBundleClient::new(&config);

        let result = client.get_bundle_statuses(vec!["test-uuid".to_string()]).await;

        assert!(result.is_ok());
        let statuses = result.unwrap();
        assert!(statuses["value"].is_array());
        assert_eq!(statuses["value"][0]["status"], "Landed");
    }
}
