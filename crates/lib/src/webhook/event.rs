use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum WebhookEvent {
    #[serde(rename = "transaction.signed")]
    TransactionSigned(TransactionSignedData),
    
    #[serde(rename = "transaction.failed")]
    TransactionFailed(TransactionFailedData),
    
    #[serde(rename = "rate_limit.hit")]
    RateLimitHit(RateLimitHitData),
}

impl WebhookEvent {
    pub fn event_type(&self) -> &str {
        match self {
            WebhookEvent::TransactionSigned(_) => "transaction.signed",
            WebhookEvent::TransactionFailed(_) => "transaction.failed",
            WebhookEvent::RateLimitHit(_) => "rate_limit.hit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSignedData {
    pub transaction_id: String,
    pub signer_pubkey: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFailedData {
    pub error: String,
    pub method: String,
    pub signer_pubkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitHitData {
    pub identifier: String,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    #[serde(flatten)]
    pub event: WebhookEvent,
    pub timestamp: DateTime<Utc>,
}

impl WebhookPayload {
    pub fn new(event: WebhookEvent) -> Self {
        Self {
            event,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type() {
        let event = WebhookEvent::TransactionSigned(TransactionSignedData {
            transaction_id: "test".to_string(),
            signer_pubkey: "pubkey".to_string(),
            method: "signTransaction".to_string(),
        });
        assert_eq!(event.event_type(), "transaction.signed");
    }

    #[test]
    fn test_payload_serialization() {
        let event = WebhookEvent::TransactionSigned(TransactionSignedData {
            transaction_id: "test_tx".to_string(),
            signer_pubkey: "test_pubkey".to_string(),
            method: "signTransaction".to_string(),
        });
        let payload = WebhookPayload::new(event);
        
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("transaction.signed"));
        assert!(json.contains("test_tx"));
        assert!(json.contains("timestamp"));
    }
}