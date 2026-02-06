use super::events::{WebhookEvent, WebhookPayload};
use crate::state::get_config;
use hmac::{Hmac, Mac};
use log::{error, info, warn};
use reqwest::Client;
use sha2::Sha256;
use std::time::Duration;
use tokio::time::sleep;

type HmacSha256 = Hmac<Sha256>;

pub async fn emit_event(event: WebhookEvent) {
    let config = match get_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to get config for webhook: {}", e);
            return;
        }
    };

    let webhook_config = &config.kora.webhook;

    if !webhook_config.enabled {
        return;
    }

    if !webhook_config.is_event_enabled(event.event_type()) {
        return;
    }

    let url = match &webhook_config.url {
        Some(u) => u.clone(),
        None => {
            error!("Webhook URL not configured");
            return;
        }
    };

    let secret = match &webhook_config.secret {
        Some(s) => s.clone(),
        None => {
            error!("Webhook secret not configured");
            return;
        }
    };

    let timeout_ms = webhook_config.timeout_ms;
    let retry_attempts = webhook_config.retry_attempts;

    // Spawn async task so webhook doesn't block the RPC response
    tokio::spawn(async move {
        send_webhook(url, event, secret, timeout_ms, retry_attempts).await;
    });
}

async fn send_webhook(
    url: String,
    event: WebhookEvent,
    secret: String,
    timeout_ms: u64,
    retry_attempts: u32,
) {
    let payload = WebhookPayload::new(event);
    
    let payload_json = match serde_json::to_string(&payload) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize webhook payload: {}", e);
            return;
        }
    };

    // Generate HMAC signature
    let signature = match generate_signature(&payload_json, &secret) {
        Ok(sig) => sig,
        Err(e) => {
            error!("Failed to generate webhook signature: {}", e);
            return;
        }
    };

    let client = Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .unwrap_or_else(|_| Client::new());

    let mut attempt = 0;
    let mut backoff = 1000; // Start with 1 second

    while attempt <= retry_attempts {
        attempt += 1;

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Timestamp", payload.timestamp.timestamp().to_string())
            .body(payload_json.clone())
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    info!(
                        "Webhook delivered successfully to {} for event {}",
                        url,
                        payload.event.event_type()
                    );
                    return;
                } else {
                    warn!(
                        "Webhook delivery failed with status {}: {} (attempt {}/{})",
                        response.status(),
                        url,
                        attempt,
                        retry_attempts + 1
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Webhook delivery error: {} (attempt {}/{})",
                    e,
                    attempt,
                    retry_attempts + 1
                );
            }
        }

        // Don't sleep after the last attempt
        if attempt <= retry_attempts {
            sleep(Duration::from_millis(backoff)).await;
            backoff *= 2; // Exponential backoff
        }
    }

    error!(
        "Webhook delivery failed after {} attempts to {}",
        retry_attempts + 1,
        url
    );
}

fn generate_signature(payload: &str, secret: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid secret key: {}", e))?;
    
    mac.update(payload.as_bytes());
    
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_signature() {
        let payload = r#"{"event":"test","timestamp":"2024-01-01T00:00:00Z"}"#;
        let secret = "test-secret";
        
        let signature = generate_signature(payload, secret).unwrap();
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64); // SHA256 produces 64 hex chars
        
        // Same input should produce same signature
        let signature2 = generate_signature(payload, secret).unwrap();
        assert_eq!(signature, signature2);
        
        // Different secret should produce different signature
        let signature3 = generate_signature(payload, "different-secret").unwrap();
        assert_ne!(signature, signature3);
    }

    #[test]
    fn test_generate_signature_invalid_secret() {
        let payload = "test";
        let secret = "";
        
        // Empty secret should still work (HMAC accepts any length)
        let result = generate_signature(payload, secret);
        assert!(result.is_ok());
    }
}