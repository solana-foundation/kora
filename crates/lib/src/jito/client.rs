//! Jito block engine HTTP client
//!
//! Provides async methods to interact with the Jito block engine API
//! for submitting bundles and checking bundle status.

use crate::{
    error::KoraError,
    jito::types::{BundleStatus, BundleStatusType, SendBundleResponse},
    sanitize_error,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Client for interacting with the Jito block engine
#[derive(Clone)]
pub struct JitoClient {
    client: Client,
    block_engine_url: String,
}

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest<T: Serialize> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: T,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error structure
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Result from sendBundle call
#[derive(Debug, Deserialize)]
struct SendBundleResult(String);

/// Result from getBundleStatuses call
#[derive(Debug, Deserialize)]
struct GetBundleStatusesResult {
    value: Vec<BundleStatusValue>,
}

#[derive(Debug, Deserialize)]
struct BundleStatusValue {
    bundle_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    transactions: Vec<String>,
    #[serde(default)]
    slot: Option<u64>,
    confirmation_status: Option<String>,
    err: Option<serde_json::Value>,
}

impl JitoClient {
    /// Creates a new Jito client
    pub fn new(block_engine_url: &str) -> Self {
        Self {
            client: Client::new(),
            block_engine_url: block_engine_url.trim_end_matches('/').to_string(),
        }
    }

    /// Creates a new Jito client with a custom reqwest client
    pub fn with_client(block_engine_url: &str, client: Client) -> Self {
        Self { client, block_engine_url: block_engine_url.trim_end_matches('/').to_string() }
    }

    /// Returns the bundles API endpoint URL
    fn bundles_url(&self) -> String {
        format!("{}/api/v1/bundles", self.block_engine_url)
    }

    /// Sends a bundle of transactions to the Jito block engine
    ///
    /// # Arguments
    /// * `transactions` - Array of base64-encoded signed transactions
    ///
    /// # Returns
    /// * `bundle_id` - The bundle ID for status tracking
    pub async fn send_bundle(
        &self,
        transactions: &[String],
    ) -> Result<SendBundleResponse, KoraError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "sendBundle",
            params: (transactions, json!({ "encoding": "base64" })),
        };

        let response = self
            .client
            .post(self.bundles_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to send bundle to Jito: {}",
                    sanitize_error!(e)
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KoraError::InternalServerError(format!(
                "Jito sendBundle failed with status {}: {}",
                status, body
            )));
        }

        let json_response: JsonRpcResponse<SendBundleResult> =
            response.json().await.map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to parse Jito response: {}",
                    sanitize_error!(e)
                ))
            })?;

        if let Some(error) = json_response.error {
            return Err(KoraError::InternalServerError(format!(
                "Jito RPC error ({}): {}",
                error.code, error.message
            )));
        }

        let result = json_response.result.ok_or_else(|| {
            KoraError::InternalServerError("Jito response missing result".to_string())
        })?;

        Ok(SendBundleResponse { bundle_id: result.0 })
    }

    /// Gets the status of one or more bundles
    ///
    /// # Arguments
    /// * `bundle_ids` - Array of bundle IDs to check
    ///
    /// # Returns
    /// * Vector of bundle statuses
    pub async fn get_bundle_statuses(
        &self,
        bundle_ids: &[String],
    ) -> Result<Vec<BundleStatus>, KoraError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getBundleStatuses",
            params: (bundle_ids,),
        };

        let response = self
            .client
            .post(self.bundles_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to get bundle statuses from Jito: {}",
                    sanitize_error!(e)
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KoraError::InternalServerError(format!(
                "Jito getBundleStatuses failed with status {}: {}",
                status, body
            )));
        }

        let json_response: JsonRpcResponse<GetBundleStatusesResult> =
            response.json().await.map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to parse Jito response: {}",
                    sanitize_error!(e)
                ))
            })?;

        if let Some(error) = json_response.error {
            return Err(KoraError::InternalServerError(format!(
                "Jito RPC error ({}): {}",
                error.code, error.message
            )));
        }

        let result = json_response.result.ok_or_else(|| {
            KoraError::InternalServerError("Jito response missing result".to_string())
        })?;

        let statuses: Vec<BundleStatus> = result
            .value
            .into_iter()
            .map(|v| {
                let status = match (v.confirmation_status.as_deref(), &v.err) {
                    (Some("confirmed") | Some("finalized"), None) => BundleStatusType::Landed,
                    (_, Some(_)) => BundleStatusType::Failed,
                    (Some("processed"), None) => BundleStatusType::Pending,
                    (None, None) => BundleStatusType::Invalid,
                    _ => BundleStatusType::Pending,
                };

                BundleStatus { bundle_id: v.bundle_id, status, landed_slot: v.slot }
            })
            .collect();

        Ok(statuses)
    }

    /// Gets the list of Jito tip accounts
    pub async fn get_tip_accounts(&self) -> Result<Vec<String>, KoraError> {
        let request =
            JsonRpcRequest { jsonrpc: "2.0", id: 1, method: "getTipAccounts", params: () };

        let response = self
            .client
            .post(self.bundles_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to get tip accounts from Jito: {}",
                    sanitize_error!(e)
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KoraError::InternalServerError(format!(
                "Jito getTipAccounts failed with status {}: {}",
                status, body
            )));
        }

        let json_response: JsonRpcResponse<Vec<String>> = response.json().await.map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to parse Jito response: {}",
                sanitize_error!(e)
            ))
        })?;

        if let Some(error) = json_response.error {
            return Err(KoraError::InternalServerError(format!(
                "Jito RPC error ({}): {}",
                error.code, error.message
            )));
        }

        json_response.result.ok_or_else(|| {
            KoraError::InternalServerError("Jito response missing result".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jito_client_new() {
        let client = JitoClient::new("https://mainnet.block-engine.jito.wtf");
        assert_eq!(client.block_engine_url, "https://mainnet.block-engine.jito.wtf");
    }

    #[test]
    fn test_jito_client_bundles_url() {
        let client = JitoClient::new("https://mainnet.block-engine.jito.wtf");
        assert_eq!(client.bundles_url(), "https://mainnet.block-engine.jito.wtf/api/v1/bundles");
    }

    #[test]
    fn test_jito_client_trailing_slash() {
        let client = JitoClient::new("https://mainnet.block-engine.jito.wtf/");
        assert_eq!(client.block_engine_url, "https://mainnet.block-engine.jito.wtf");
    }
}
